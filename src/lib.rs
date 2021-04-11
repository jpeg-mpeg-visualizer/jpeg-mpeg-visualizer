use std::rc::Rc;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

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

            for i in (0..data.len()).step_by(4) {
                data[i + 1] = 0;
                data[i + 2] = 0;
            }

            let new_image_data =
                web_sys::ImageData::new_with_u8_clamped_array(wasm_bindgen::Clamped(&data), 500)
                    .unwrap();
            context.put_image_data(&new_image_data, 0.0, 0.0).unwrap();
        }) as Box<dyn FnMut()>);
        image.set_onload(Some(imageonload.as_ref().unchecked_ref()));
        image.set_src(&src);
        imageonload.forget();
    }) as Box<dyn FnMut(_)>);
    fr.set_onload(Some(onload.as_ref().unchecked_ref()));
    onload.forget();
    fr.read_as_data_url(&file).unwrap();
}
