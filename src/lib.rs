use once_cell::sync::Lazy;
use std::rc::Rc;
use std::sync::Mutex;
use wasm_bindgen::prelude::*;
use wasm_bindgen::{JsCast, Clamped};

mod dct;
mod quant;

#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

struct RGB((u8, u8, u8));

impl RGB {
    fn to_ycbcr(&self) -> YCbCr {
        let (r, g, b) = self.0;
        let (r, g, b) = (r as f32, g as f32, b as f32);

        let y = 0.299 * r + 0.587 * g + 0.114 * b;
        let cb = 128.0 - 0.168736 * r - 0.331264 * g + 0.5 * b;
        let cr = 128.0 + 0.5 * r - 0.418688 * g - 0.081312 * b;
        YCbCr((y as u8, cb as u8, cr as u8))
    }
}

struct YCbCr((u8, u8, u8));

impl YCbCr {
    fn to_rgb(&self) -> RGB {
        let (y, cb, cr) = self.0;
        let (y, cb, cr) = (y as f32, cb as f32, cr as f32);

        let r = y + 1.402 * (cr - 128.0);
        let g = y - 0.344136 * (cb - 128.0) - 0.714136 * (cr - 128.0);
        let b = y + 1.772 * (cb - 128.0);
        RGB((r as u8, g as u8, b as u8))
    }
}

#[derive(Default)]
struct JPEGState {
    image_data: Vec<u8>,
    ycbcr: Option<Vec<YCbCr>>,
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
            STATE.lock().unwrap().image_data = data;
            get_ycbcr();
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
    drop(state_lock);
    draw_spatial();
}

fn get_ycbcr() {
    let mut state_lock = STATE.lock().unwrap();
    let data = &state_lock.image_data;

    let mut ycbcr = Vec::new();
    for i in (0..data.len()).step_by(4) {
        let r = data[i];
        let g = data[i + 1];
        let b = data[i + 2];

        let tuple = RGB((r, g, b)).to_ycbcr();
        ycbcr.push(tuple);
    }
    state_lock.ycbcr = Some(ycbcr);
    drop(state_lock);

    draw_ycbcr();
    draw_spatial();
}

fn draw_ycbcr() {
    let state_lock = STATE.lock().unwrap();
    let ycbcr = state_lock.ycbcr.as_ref().unwrap();

    let ys = ycbcr
        .iter()
        .flat_map(|x| {
            let (r, g, b) = YCbCr((x.0 .0, 128, 128)).to_rgb().0;
            vec![r, g, b, 255]
        })
        .collect::<Vec<u8>>();

    let cbs = ycbcr
        .iter()
        .flat_map(|x| {
            let (r, g, b) = YCbCr((128, x.0 .1, 128)).to_rgb().0;
            vec![r, g, b, 255]
        })
        .collect::<Vec<u8>>();

    let crs = ycbcr
        .iter()
        .flat_map(|x| {
            let (r, g, b) = YCbCr((128, 128, x.0 .2)).to_rgb().0;
            vec![r, g, b, 255]
        })
        .collect::<Vec<u8>>();

    let y = get_canvas_context("y");
    let cb = get_canvas_context("cb");
    let cr = get_canvas_context("cr");

    let ys =
        web_sys::ImageData::new_with_u8_clamped_array(wasm_bindgen::Clamped(&ys), 500).unwrap();
    y.put_image_data(&ys, 0.0, 0.0).unwrap();
    let cbs =
        web_sys::ImageData::new_with_u8_clamped_array(wasm_bindgen::Clamped(&cbs), 500).unwrap();
    cb.put_image_data(&cbs, 0.0, 0.0).unwrap();
    let crs =
        web_sys::ImageData::new_with_u8_clamped_array(wasm_bindgen::Clamped(&crs), 500).unwrap();
    cr.put_image_data(&crs, 0.0, 0.0).unwrap();
}

fn draw_spatial() {
    let state_lock = STATE.lock().unwrap();
    let ycbcr = state_lock.ycbcr.as_ref().unwrap();
    let quality = state_lock.quality;

    let ys = ycbcr.iter().map(|x| x.0 .0).collect::<Vec<u8>>();
    let cbs = ycbcr.iter().map(|x| x.0 .1).collect::<Vec<u8>>();
    let crs = ycbcr.iter().map(|x| x.0 .2).collect::<Vec<u8>>();

    let y_context = get_canvas_context("y_spatial");
    let cb_context = get_canvas_context("cb_spatial");
    let cr_context = get_canvas_context("cr_spatial");

    draw_spatial_channel(&ys, &y_context, quality, true);
    draw_spatial_channel(&cbs, &cb_context, quality, false);
    draw_spatial_channel(&crs, &cr_context, quality, false);
}

fn draw_spatial_channel(
    data: &Vec<u8>,
    canvas_context: &web_sys::CanvasRenderingContext2d,
    quality: u8,
    luminance: bool,
) {
    let mut image_data = vec![0; 500 * 500 * 4];
    let block_count = data.len() / (8 * 500);

    for v in 0..block_count {
        for u in 0..block_count {
            let block = get_block(u, v, &data);
            let mut spatial = dct::spatial_to_freq(&block);
            quant::apply_quantization(&mut spatial, quality, luminance);
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

fn write_to_image_data(image_data: &mut Vec<u8>, spatial: &Vec<Vec<u8>>, u: usize, v: usize) {
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

#[wasm_bindgen]
pub fn choose_block(global_x: u32, global_y: u32) {
    let window = web_sys::window().expect("no global `window` exists");
    let document = window.document().expect("should have a document on window");

    let canvas = document.get_element_by_id("canvas").unwrap();
    canvas.set_class_name("");
    let canvas = canvas.dyn_into::<web_sys::HtmlCanvasElement>().unwrap();
    let rect = canvas.get_bounding_client_rect();
    // Get rid of offset (due to canvas posistioning) and calculate the top-left corner of chosen block
    let x: u32 = ((global_x - rect.left() as u32)/8)*8;
    let y: u32 = ((global_y - rect.top() as u32)/8)*8;

    let ctx = get_canvas_context("canvas");

    // Reload original image (to get rid of previous choice rectangle)
    let state_lock = STATE.lock().unwrap();
    ctx.put_image_data(&web_sys::ImageData::new_with_u8_clamped_array(Clamped(&state_lock.image_data), 500).unwrap(), 0.0, 0.0).unwrap();

    ctx.begin_path();
    ctx.rect(x as f64, y as f64, 8.0, 8.0);
    ctx.stroke();
}

fn get_block(u: usize, v: usize, data: &Vec<u8>) -> Vec<Vec<u8>> {
    let mut result = vec![vec![0; 8]; 8];

    for y in 0..8 {
        for x in 0..8 {
            result[y][x] = data[(v * 8 + y) * 500 + (u * 8) + x];
        }
    }

    result
}
