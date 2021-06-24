use seed::JsFuture;
use wasm_bindgen::JsCast;

use crate::image;

pub(super) async fn load_image(file_blob: gloo_file::Blob) -> image::RawImage {
    let url_data = gloo_file::futures::read_as_data_url(&file_blob)
        .await
        .unwrap();
    let image = web_sys::HtmlImageElement::new().unwrap();
    image.set_src(&url_data);
    JsFuture::from(image.decode()).await;
    let height = image.natural_height();
    let width = image.natural_width();
    let canvas = web_sys::window()
        .unwrap()
        .document()
        .unwrap()
        .create_element("canvas")
        .unwrap()
        .dyn_into::<web_sys::HtmlCanvasElement>()
        .unwrap();
    canvas.set_width(width);
    canvas.set_height(height);
    let context = canvas
        .get_context("2d")
        .unwrap()
        .unwrap()
        .dyn_into::<web_sys::CanvasRenderingContext2d>()
        .unwrap();
    context
        .draw_image_with_html_image_element(&image, 0.0, 0.0)
        .unwrap();
    let image_data = context
        .get_image_data(0.0, 0.0, width.into(), height.into())
        .unwrap();
    let data: Vec<u8> = image_data.data().to_vec();
    image::RawImage::new(data, width, height)
}
