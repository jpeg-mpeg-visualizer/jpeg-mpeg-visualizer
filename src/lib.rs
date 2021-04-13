use once_cell::sync::Lazy;
use std::rc::Rc;
use std::sync::Mutex;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

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
        let r = self.0 .0;
        let g = self.0 .1;
        let b = self.0 .2;

        let y = 0.299 * r as f32 + 0.587 * g as f32 + 0.114 * b as f32;
        let cb = 128.0 - 0.168736 * r as f32 + 0.331264 * g as f32 + 0.5 * b as f32;
        let cr = 128.0 + 0.5 * r as f32 + 0.418688 * g as f32 + 0.081312 * b as f32;
        YCbCr((y as u8, cb as u8, cr as u8))
    }
}

struct YCbCr((u8, u8, u8));

impl YCbCr {
    fn to_rgb(&self) -> RGB {
        let y = self.0 .0;
        let cb = self.0 .1;
        let cr = self.0 .2;

        let r = y as f32 + 1.402 * (cr as f32 - 128.0);
        let g = y as f32 - 0.344136 * (cb as f32 - 128.0) - 0.714136 * (cr as f32 - 128.0);
        let b = y as f32 + 1.772 * (cb as f32 - 128.0);
        RGB((r as u8, g as u8, b as u8))
    }
}

#[derive(Default)]
struct JPEGState {
    image_data: Vec<u8>,
    ycbcr: Option<Vec<YCbCr>>,
}

static state: Lazy<Mutex<JPEGState>> = Lazy::new(|| Mutex::new(Default::default()));

#[wasm_bindgen(start)]
pub fn main() -> Result<(), JsValue> {
    let window = web_sys::window().expect("no global `window` exists");
    let document = window.document().expect("should have a document on window");
    let body = document.body().expect("document should have a body");

    // Manufacture the element we're gonna append
    let val = document.create_element("p")?;
    val.set_inner_html("Hello from Rust!");

    body.append_child(&val)?;

    Ok(())
}

#[wasm_bindgen]
pub fn load_img(file: web_sys::File) {
    let document = web_sys::window().unwrap().document().unwrap();
    let canvas = document.get_element_by_id("canvas").unwrap();
    let canvas = canvas.dyn_into::<web_sys::HtmlCanvasElement>().unwrap();

    let fr = web_sys::FileReader::new().unwrap();

    let onload = Closure::wrap(Box::new(move |e: web_sys::ProgressEvent| {
        let context = canvas
            .get_context("2d")
            .unwrap()
            .unwrap()
            .dyn_into::<web_sys::CanvasRenderingContext2d>()
            .unwrap();

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
            let mut data: Vec<u8> = image_data.data().to_vec();

            // for i in (0..data.len()).step_by(4) {
            //     data[i + 1] = 0;
            //     data[i + 2] = 0;
            // }

            let new_image_data =
                web_sys::ImageData::new_with_u8_clamped_array(wasm_bindgen::Clamped(&data), 500)
                    .unwrap();
            context.put_image_data(&new_image_data, 0.0, 0.0).unwrap();
            state.lock().unwrap().image_data = data;
            get_ycbcr();
        }) as Box<dyn FnMut()>);
        image.set_onload(Some(imageonload.as_ref().unchecked_ref()));
        image.set_src(&src);
        imageonload.forget();
    }) as Box<dyn FnMut(_)>);
    fr.set_onload(Some(onload.as_ref().unchecked_ref()));
    onload.forget();
    fr.read_as_data_url(&file).unwrap();
}

fn get_ycbcr() {
    let mut state_lock = state.lock().unwrap();
    let data = &state_lock.image_data;

    let mut ycbcr = Vec::new();
    for i in (0..data.len()).step_by(4) {
        let r = data[i];
        let g = data[i + 1];
        let b = data[i + 2];

        // let y = 0.299 * r as f32 + 0.587 * g as f32 + 0.114 * b as f32;
        // let cb = 128.0.299 * r as f32 + 0.587 * g as f32 + 0.114 * b as f32;
        // let cb = 0;
        // let cr = 0;

        let tuple = RGB((r, g, b)).to_ycbcr();
        ycbcr.push(tuple);
    }
    state_lock.ycbcr = Some(ycbcr);
    drop(state_lock);

    draw_ycbcr();
}

fn draw_ycbcr() {
    let window = web_sys::window().expect("no global `window` exists");
    let document = window.document().expect("should have a document on window");
    let body = document.body().expect("document should have a body");

    let y = document.get_element_by_id("y").unwrap();
    let y = y.dyn_into::<web_sys::HtmlCanvasElement>().unwrap();
    y.set_class_name("");

    let cb = document.get_element_by_id("cb").unwrap();
    let cb = cb.dyn_into::<web_sys::HtmlCanvasElement>().unwrap();
    cb.set_class_name("");

    let cr = document.get_element_by_id("cr").unwrap();
    let cr = cr.dyn_into::<web_sys::HtmlCanvasElement>().unwrap();
    cr.set_class_name("");

    let state_lock = state.lock().unwrap();
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

    // let ys = [1];
    // log("test");

    // log(&ys.len().to_string());

    let y = y
        .get_context("2d")
        .unwrap()
        .unwrap()
        .dyn_into::<web_sys::CanvasRenderingContext2d>()
        .unwrap();

    let cb = cb
        .get_context("2d")
        .unwrap()
        .unwrap()
        .dyn_into::<web_sys::CanvasRenderingContext2d>()
        .unwrap();

    let cr = cr
        .get_context("2d")
        .unwrap()
        .unwrap()
        .dyn_into::<web_sys::CanvasRenderingContext2d>()
        .unwrap();

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
