use once_cell::sync::Lazy;
use std::rc::Rc;
use std::sync::Mutex;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use crate::quant::{scale_quantization_table, LUMINANCE_QUANTIZATION_TABLE, CHROMINANCE_QUANTIZATION_TABLE, BlockMatrix};

mod dct;
mod quant;
mod image;
mod pixel;

#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}



#[derive(Default)]
struct JPEGState {
    image_data: image::RawImage,
    ycbcr: Option<image::YCbCrImage>,
    rgb: Option<image::RGBImage>,
    quality: u8,
}

static STATE: Lazy<Mutex<JPEGState>> = Lazy::new(|| {
    Mutex::new(JPEGState {
        quality: 50,
        ..Default::default()
    })
});

#[wasm_bindgen(start)]
pub fn main() -> Result<(), JsValue> {
    console_error_panic_hook::set_once();
    Ok(())
}

fn get_canvas_context(id: &str) -> web_sys::CanvasRenderingContext2d {
    let window = web_sys::window().expect("no global `window` exists");
    let document = window.document().expect("should have a document on window");

    let canvas = document.get_element_by_id(id).unwrap();
    canvas.set_class_name("");
    let canvas = canvas.dyn_into::<web_sys::HtmlCanvasElement>().unwrap();

    canvas
        .get_context("2d")
        .unwrap()
        .unwrap()
        .dyn_into::<web_sys::CanvasRenderingContext2d>()
        .unwrap()
}

#[wasm_bindgen]
pub fn load_img(file: web_sys::File) {
    let fr = web_sys::FileReader::new().unwrap();

    let onload = Closure::wrap(Box::new(move |e: web_sys::ProgressEvent| {
        let context = get_canvas_context("canvas");

        let image = Rc::new(web_sys::HtmlImageElement::new().unwrap());
        let t = e.target().unwrap();
        let fr: &web_sys::FileReader = t.unchecked_ref();
        let src = fr.result().unwrap().as_string().unwrap();

        let image_copy = Rc::clone(&image);

        let imageonload = Closure::wrap(Box::new(move || {
            context
                .draw_image_with_html_image_element(&image_copy, 0.0, 0.0)
                .unwrap();

            let image_data = context.get_image_data(0.0, 0.0, 500.0, 500.0).unwrap();
            let data: Vec<u8> = image_data.data().to_vec();

            context.put_image_data(&image_data, 0.0, 0.0).unwrap();
            STATE.lock().unwrap().image_data = image::RawImage(data);
            process_image();
        }) as Box<dyn FnMut()>);

        image.set_onload(Some(imageonload.as_ref().unchecked_ref()));
        imageonload.forget();
        image.set_src(&src);
    }) as Box<dyn FnMut(_)>);

    fr.set_onload(Some(onload.as_ref().unchecked_ref()));
    onload.forget();
    fr.read_as_data_url(&file).unwrap();
}

#[wasm_bindgen]
pub fn set_quality(quality: u8) {
    let mut state_lock = STATE.lock().unwrap();
    state_lock.quality = quality;

    let ys = state_lock.ycbcr.as_ref().unwrap().to_ys_channel();
    let cbs = state_lock.ycbcr.as_ref().unwrap().to_cbs_channel();
    let crs = state_lock.ycbcr.as_ref().unwrap().to_crs_channel();

    drop(state_lock);
    process_spatial();
}

fn process_spatial() {
    let mut state_lock = STATE.lock().unwrap();

    let ys = state_lock.ycbcr.as_ref().unwrap().to_ys_channel();
    let cbs = state_lock.ycbcr.as_ref().unwrap().to_cbs_channel();
    let crs = state_lock.ycbcr.as_ref().unwrap().to_crs_channel();

    let quality = state_lock.quality;

    let scaled_luminance_quant_table =
        scale_quantization_table(&LUMINANCE_QUANTIZATION_TABLE, quality);
    let scaled_chrominance_quant_table =
        scale_quantization_table(&CHROMINANCE_QUANTIZATION_TABLE, quality);

    let ys_block_matrix = quant::split_to_block_matrix(&ys);
    let cbs_block_matrix = quant::split_to_block_matrix(&cbs);
    let crs_block_matrix = quant::split_to_block_matrix(&crs);

    let ys_quantized = ys_block_matrix.apply_quantization(&scaled_luminance_quant_table);
    let cbs_quantized = cbs_block_matrix.apply_quantization(&scaled_chrominance_quant_table);
    let crs_quantized = crs_block_matrix.apply_quantization(&scaled_chrominance_quant_table);

    draw_spatial(
        &ys_quantized,
        &cbs_quantized,
        &crs_quantized,
        crs_block_matrix.width,
        crs_block_matrix.height
    );
}

fn process_image() {
    let mut state_lock = STATE.lock().unwrap();

    state_lock.rgb = Some(state_lock.image_data.to_rgb_image());
    state_lock.ycbcr = Some(state_lock.rgb.as_ref().unwrap().to_ycbcr_image());

    let ys = state_lock.ycbcr.as_ref().unwrap().to_ys_channel();
    let cbs = state_lock.ycbcr.as_ref().unwrap().to_cbs_channel();
    let crs = state_lock.ycbcr.as_ref().unwrap().to_crs_channel();

    let quality = state_lock.quality;

    drop(state_lock);

    draw_ycbcr(&ys, &cbs, &crs);

    process_spatial();
}

fn draw_ycbcr(ys: &Vec<u8>, cbs: &Vec<u8>, crs: &Vec<u8>) {

    let ys_image = ys
        .iter()
        .flat_map(|x| {
            let (r, g, b) = pixel::YCbCr((*x, 128, 128)).to_rgb().0;
            vec![r, g, b, 255]
        })
        .collect::<Vec<u8>>();

    let cbs_image = cbs
        .iter()
        .flat_map(|x| {
            let (r, g, b) = pixel::YCbCr((128, *x, 128)).to_rgb().0;
            vec![r, g, b, 255]
        })
        .collect::<Vec<u8>>();

    let crs_image = crs
        .iter()
        .flat_map(|x| {
            let (r, g, b) = pixel::YCbCr((128, 128, *x)).to_rgb().0;
            vec![r, g, b, 255]
        })
        .collect::<Vec<u8>>();

    let y = get_canvas_context("y");
    let cb = get_canvas_context("cb");
    let cr = get_canvas_context("cr");

    let ys =
        web_sys::ImageData::new_with_u8_clamped_array(wasm_bindgen::Clamped(&ys_image), 500).unwrap();
    y.put_image_data(&ys, 0.0, 0.0).unwrap();
    let cbs =
        web_sys::ImageData::new_with_u8_clamped_array(wasm_bindgen::Clamped(&cbs_image), 500).unwrap();
    cb.put_image_data(&cbs, 0.0, 0.0).unwrap();
    let crs =
        web_sys::ImageData::new_with_u8_clamped_array(wasm_bindgen::Clamped(&crs_image), 500).unwrap();
    cr.put_image_data(&crs, 0.0, 0.0).unwrap();
}

fn draw_spatial(
    ys: &Vec<[[u8; 8]; 8]>,
    cbs: &Vec<[[u8; 8]; 8]>,
    crs: &Vec<[[u8; 8]; 8]>,
    width: usize,
    height: usize,
) {

    let y_context = get_canvas_context("y_spatial");
    let cb_context = get_canvas_context("cb_spatial");
    let cr_context = get_canvas_context("cr_spatial");

    draw_spatial_channel(&ys, width, height, &y_context);
    draw_spatial_channel(&cbs, width, height, &cb_context);
    draw_spatial_channel(&crs, width, height, &cr_context);
}

fn draw_spatial_channel(
    data: &Vec<[[u8; 8]; 8]>,
    width: usize,
    height: usize,
    canvas_context: &web_sys::CanvasRenderingContext2d,
) {
    let mut image_data = vec![0; 500 * 500 * 4];

    for v in 0..width {
        for u in 0..height {
            let mut spatial = data[u + v * width];
            write_to_image_data(&mut image_data, &spatial, u, v);
        }
    }

    let image_data =
        web_sys::ImageData::new_with_u8_clamped_array(wasm_bindgen::Clamped(&image_data), 500)
            .unwrap();

    canvas_context
        .put_image_data(&image_data, 0.0, 0.0)
        .unwrap();
}

fn write_to_image_data(image_data: &mut Vec<u8>, spatial: &[[u8; 8]; 8], u: usize, v: usize) {
    for y in 0..8 {
        for x in 0..8 {
            let offset = ((v * 8 + y) * 500 + (u * 8) + x) * 4;
            image_data[offset] = 255 - spatial[y][x];
            image_data[offset + 1] = 255 - spatial[y][x];
            image_data[offset + 2] = 255 - spatial[y][x];
            image_data[offset + 3] = 255;
        }
    }
}


