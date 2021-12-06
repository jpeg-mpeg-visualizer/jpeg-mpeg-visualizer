use seed::prelude::*;
use seed::*;

use crate::section::jpeg_visualization::utils::create_tmp_canvas;
use web_sys::{HtmlCanvasElement, ImageData};

// TODO: Consider having seperate canvas for "scalable" images and another one with w/h set at static context
// TMP_CANVAS so that we can draw scaled image to proper canvases
// TODO: Get one global canvas to work when there will be time for that...
/*static TMP_CANVAS: Lazy<HtmlCanvasElement> = Lazy::new(|| {
    let canvas = web_sys::window()
        .unwrap()
        .document()
        .unwrap()
        .create_element("canvas")
        .unwrap()
        .dyn_into::<web_sys::HtmlCanvasElement>()
        .unwrap();
    canvas
});*/

// TODO: Consider scaling all canvases once - would have to be done on init but would get rid of passing scale here and would mean less operations
pub fn draw_scaled_image_with_image_data_with_w_h_and_scale(
    canvas: &ElRef<HtmlCanvasElement>,
    image_data: &ImageData,
    width: u32,
    height: u32,
    scale_x: f64,
    scale_y: f64,
) {
    let ctx = canvas_context_2d(&canvas.get().unwrap());

    let tmp_canvas = create_tmp_canvas();

    tmp_canvas.set_width(width);
    tmp_canvas.set_height(height);
    let tmp_ctx = canvas_context_2d(&tmp_canvas);
    tmp_ctx.put_image_data(&image_data, 0.0, 0.0).unwrap();
    ctx.scale(scale_x, scale_y).unwrap();
    ctx.draw_image_with_html_canvas_element(&tmp_canvas, 0.0, 0.0)
        .unwrap();
    ctx.scale(1.0 / scale_x, 1.0 / scale_y).unwrap();
}

#[allow(dead_code)]
pub fn draw_scaled_image_with_w_h_and_scale(
    canvas: &ElRef<HtmlCanvasElement>,
    image: &Vec<u8>,
    width: u32,
    height: u32,
    scale_x: f64,
    scale_y: f64,
) {
    let image_data =
        web_sys::ImageData::new_with_u8_clamped_array(wasm_bindgen::Clamped(&image), width)
            .unwrap();
    draw_scaled_image_with_image_data_with_w_h_and_scale(
        &canvas,
        &image_data,
        width,
        height,
        scale_x,
        scale_y,
    );
}

pub fn draw_scaled_image_default_with_image_data(
    canvas: &ElRef<HtmlCanvasElement>,
    image_data: &ImageData,
    zoom: u32,
) {
    draw_scaled_image_with_image_data_with_w_h_and_scale(
        &canvas,
        &image_data,
        canvas.get().unwrap().width() / zoom,
        canvas.get().unwrap().height() / zoom,
        zoom as f64,
        zoom as f64,
    );
}

pub fn draw_scaled_image_default(canvas: &ElRef<HtmlCanvasElement>, image: &Vec<u8>, zoom: u32) {
    let image_data = web_sys::ImageData::new_with_u8_clamped_array(
        wasm_bindgen::Clamped(&image),
        canvas.get().unwrap().width() / zoom,
    )
    .unwrap();
    draw_scaled_image_default_with_image_data(&canvas, &image_data, zoom);
}

pub fn clear_canvas(canvas: &ElRef<HtmlCanvasElement>) {
    let ctx = canvas_context_2d(&canvas.get().unwrap());
    ctx.clear_rect(
        0.0,
        0.0,
        canvas.get().unwrap().width() as f64,
        canvas.get().unwrap().height() as f64,
    );
}
