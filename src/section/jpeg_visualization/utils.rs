use seed::JsFuture;
use wasm_bindgen::JsCast;

use crate::image;
use crate::section::jpeg_visualization::model::SubsamplingPack;
use std::cmp;
use web_sys::HtmlCanvasElement;

pub(super) async fn load_image(file_blob: gloo_file::Blob) -> image::RawImage {
    let url_data = gloo_file::futures::read_as_data_url(&file_blob)
        .await
        .unwrap();
    let image = web_sys::HtmlImageElement::new().unwrap();
    image.set_src(&url_data);
    JsFuture::from(image.decode()).await.unwrap();
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

pub fn get_image_diff(img_a: &Vec<u8>, img_b: &Vec<u8>) -> Vec<u8> {
    assert_eq!(img_a.len(), img_b.len());
    let mut res: Vec<u8> = Vec::new();
    for i in (0..img_a.len() as usize).step_by(4) {
        let r_a = img_a[i] as i16;
        let r_b = img_b[i] as i16;
        let g_a = img_a[i + 1] as i16;
        let g_b = img_b[i + 1] as i16;
        let b_a = img_a[i + 2] as i16;
        let b_b = img_b[i + 2] as i16;
        // We don't affect alfa, so it can be equal to original
        let alfa = img_b[i + 3];
        let diff: i16 = (r_a - r_b).abs() + (g_a - g_b).abs() + (b_a - b_b).abs();
        let val: u8 = 255 - cmp::min(diff, 255) as u8;
        res.push(val);
        res.push(val);
        res.push(val);
        res.push(alfa);
    }
    res
}

pub fn create_tmp_canvas() -> HtmlCanvasElement {
    return web_sys::window()
        .unwrap()
        .document()
        .unwrap()
        .create_element("canvas")
        .unwrap()
        .dyn_into::<web_sys::HtmlCanvasElement>()
        .unwrap();
}

pub fn horiz_mult_from_subsampling(subsampling_pack: &SubsamplingPack) -> usize {
    (subsampling_pack.j / subsampling_pack.a) as usize
}
pub fn vert_mult_from_subsampling(subsampling_pack: &SubsamplingPack) -> usize {
    if subsampling_pack.b == 0 {
        return 2_usize;
    } else {
        return 1_usize;
    }
}
