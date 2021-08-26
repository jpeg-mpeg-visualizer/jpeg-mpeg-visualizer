use seed::prelude::*;
use seed::*;
use std::cmp;

use super::model::*;
use super::utils;
use super::view::*;
use crate::image::pixel::RGB;
use crate::image::RawImageWindow;
use crate::{block, image, quant, Msg as GMsg, BLOCK_SIZE, ZOOM};
use std::rc::Rc;
use web_sys::HtmlCanvasElement;
use web_sys::HtmlDivElement;

pub fn init(url: Url) -> Option<Model> {
    let base_url = url.to_base_url();

    Some(Model {
        file_chooser_zone_active: false,
        base_url,
        state: State::FileChooser,
        original_canvas_preview: ElRef::<HtmlCanvasElement>::default(),
        original_canvas: ElRef::<HtmlCanvasElement>::default(),
        original_canvas_scrollable_div_wrapper: ElRef::<HtmlDivElement>::default(),
        ys_canvas: ElRef::<HtmlCanvasElement>::default(),
        cbs_canvas: ElRef::<HtmlCanvasElement>::default(),
        crs_canvas: ElRef::<HtmlCanvasElement>::default(),
        ys_quant_canvas: ElRef::<HtmlCanvasElement>::default(),
        cbs_quant_canvas: ElRef::<HtmlCanvasElement>::default(),
        crs_quant_canvas: ElRef::<HtmlCanvasElement>::default(),

        quality: 50,
    })
}

// ------ ------
//      View
// ------ ------

pub fn view(model: &Model) -> Node<GMsg> {
    match &model.state {
        State::FileChooser => view_file_chooser(model),
        State::PreImageView => view_jpeg_visualization(model),
        State::ImageView(_raw_image) => view_jpeg_visualization(model),
    }
}

pub fn wrap(msg: Msg) -> GMsg {
    GMsg::JPEGVisualizationMessage(msg)
}

fn draw_original_image_preview(
    original_canvas_preview: &ElRef<HtmlCanvasElement>,
    image: &image::RawImage,
) {
    let canvas = original_canvas_preview.get().unwrap();
    let ctx = canvas_context_2d(&canvas);
    let img = web_sys::ImageData::new_with_u8_clamped_array(
        wasm_bindgen::Clamped(image.as_ref()),
        image.width(),
    )
    .unwrap();

    // Create temporary canvas1 so that we can draw scaled image to proper canvas
    let tmp_canvas = web_sys::window()
        .unwrap()
        .document()
        .unwrap()
        .create_element("canvas")
        .unwrap()
        .dyn_into::<web_sys::HtmlCanvasElement>()
        .unwrap();
    tmp_canvas.set_height(image.height());
    tmp_canvas.set_width(image.width());
    let tmp_ctx = canvas_context_2d(&tmp_canvas);
    tmp_ctx.put_image_data(&img, 0.0, 0.0).unwrap();

    // Set scale and draw scaled image from temporary canvas1
    ctx.scale(
        (BLOCK_SIZE * ZOOM) as f64 / image.width() as f64,
        (BLOCK_SIZE * ZOOM) as f64 / image.height() as f64,
    )
    .unwrap();

    ctx.draw_image_with_html_canvas_element(&tmp_canvas, 0.0, 0.0)
        .unwrap();
}
fn draw_original_image(canvas: &ElRef<HtmlCanvasElement>, image: &image::RawImage) {
    let canvas = canvas.get().unwrap();
    canvas.set_height(image.height() * ZOOM);
    canvas.set_width(image.width() * ZOOM);
    let ctx = canvas_context_2d(&canvas);
    let img = web_sys::ImageData::new_with_u8_clamped_array(
        wasm_bindgen::Clamped(image.as_ref()),
        image.width(),
    )
    .unwrap();

    // Create temporary canvas1 so that we can draw scaled image to proper canvas
    let tmp_canvas = web_sys::window()
        .unwrap()
        .document()
        .unwrap()
        .create_element("canvas")
        .unwrap()
        .dyn_into::<web_sys::HtmlCanvasElement>()
        .unwrap();
    tmp_canvas.set_height(image.height());
    tmp_canvas.set_width(image.width());
    let tmp_ctx = canvas_context_2d(&tmp_canvas);
    tmp_ctx.put_image_data(&img, 0.0, 0.0).unwrap();

    ctx.scale(ZOOM as f64, ZOOM as f64);
    ctx.draw_image_with_html_canvas_element(&tmp_canvas, 0.0, 0.0)
        .unwrap();
}

fn draw_block_choice_indicator(
    canvas: &ElRef<HtmlCanvasElement>,
    image: &image::RawImage,
    start_x: u32,
    start_y: u32,
) {
    // Reset previous block choice indicator
    draw_original_image(canvas, image);

    let canvas = canvas.get().unwrap();
    let ctx = canvas_context_2d(&canvas);
    // Draw rect
    ctx.begin_path();
    ctx.rect(start_x.into(), start_y.into(), 16.0_f64, 16.0_f64);
    ctx.stroke();
}

fn draw_ycbcr(
    canvas_ys: &ElRef<HtmlCanvasElement>,
    canvas_cbs: &ElRef<HtmlCanvasElement>,
    canvas_crs: &ElRef<HtmlCanvasElement>,
    image: &image::YCbCrImage,
) {
    let ctx_ys = canvas_context_2d(&canvas_ys.get().unwrap());
    let ctx_cbs = canvas_context_2d(&canvas_cbs.get().unwrap());
    let ctx_crs = canvas_context_2d(&canvas_crs.get().unwrap());

    let ys = image.to_ys_channel();
    let cbs = image.to_cbs_channel();
    let crs = image.to_crs_channel();

    let ys_image = ys
        .iter()
        .flat_map(|x| {
            let RGB { r, g, b } = image::pixel::YCbCr {
                y: *x,
                cb: 128,
                cr: 128,
            }
            .to_rgb();
            vec![r, g, b, 255]
        })
        .collect::<Vec<u8>>();

    let cbs_image = cbs
        .iter()
        .flat_map(|x| {
            let RGB { r, g, b } = image::pixel::YCbCr {
                y: 128,
                cb: *x,
                cr: 128,
            }
            .to_rgb();
            vec![r, g, b, 255]
        })
        .collect::<Vec<u8>>();

    let crs_image = crs
        .iter()
        .flat_map(|x| {
            let RGB { r, g, b } = image::pixel::YCbCr {
                y: 128,
                cb: 128,
                cr: *x,
            }
            .to_rgb();
            vec![r, g, b, 255]
        })
        .collect::<Vec<u8>>();

    // Create temporary canvas1 so that we can draw scaled image to proper canvas
    let tmp_canvas = web_sys::window()
        .unwrap()
        .document()
        .unwrap()
        .create_element("canvas")
        .unwrap()
        .dyn_into::<web_sys::HtmlCanvasElement>()
        .unwrap();
    tmp_canvas.set_height(BLOCK_SIZE);
    tmp_canvas.set_width(BLOCK_SIZE);
    let tmp_ctx = canvas_context_2d(&tmp_canvas);

    let ys =
        web_sys::ImageData::new_with_u8_clamped_array(wasm_bindgen::Clamped(&ys_image), BLOCK_SIZE)
            .unwrap();
    tmp_ctx.put_image_data(&ys, 0.0, 0.0).unwrap();
    ctx_ys.scale(ZOOM as f64, ZOOM as f64).unwrap();
    ctx_ys
        .draw_image_with_html_canvas_element(&tmp_canvas, 0.0, 0.0)
        .unwrap();
    ctx_ys.scale(1.0 / ZOOM as f64, 1.0 / ZOOM as f64);

    let cbs = web_sys::ImageData::new_with_u8_clamped_array(
        wasm_bindgen::Clamped(&cbs_image),
        BLOCK_SIZE,
    )
    .unwrap();
    tmp_ctx.put_image_data(&cbs, 0.0, 0.0).unwrap();
    ctx_cbs.scale(ZOOM as f64, ZOOM as f64).unwrap();
    ctx_cbs
        .draw_image_with_html_canvas_element(&tmp_canvas, 0.0, 0.0)
        .unwrap();
    ctx_cbs.scale(1.0 / ZOOM as f64, 1.0 / ZOOM as f64);

    let crs = web_sys::ImageData::new_with_u8_clamped_array(
        wasm_bindgen::Clamped(&crs_image),
        BLOCK_SIZE,
    )
    .unwrap();
    tmp_ctx.put_image_data(&crs, 0.0, 0.0).unwrap();
    ctx_crs.scale(ZOOM as f64, ZOOM as f64).unwrap();
    ctx_crs
        .draw_image_with_html_canvas_element(&tmp_canvas, 0.0, 0.0)
        .unwrap();
    ctx_crs.scale(1.0 / ZOOM as f64, 1.0 / ZOOM as f64);
}

fn draw_dct_quantized(
    canvas_ys: &ElRef<HtmlCanvasElement>,
    canvas_cbs: &ElRef<HtmlCanvasElement>,
    canvas_crs: &ElRef<HtmlCanvasElement>,
    image: &image::YCbCrImage,
    quality: u8,
) {
    let ctx_ys = canvas_context_2d(&canvas_ys.get().unwrap());
    let ctx_cbs = canvas_context_2d(&canvas_cbs.get().unwrap());
    let ctx_crs = canvas_context_2d(&canvas_crs.get().unwrap());

    let ys = image.to_ys_channel();
    let cbs = image.to_cbs_channel();
    let crs = image.to_crs_channel();

    let scaled_luminance_quant_table =
        quant::scale_quantization_table(&quant::LUMINANCE_QUANTIZATION_TABLE, quality);
    let scaled_chrominance_quant_table =
        quant::scale_quantization_table(&quant::CHROMINANCE_QUANTIZATION_TABLE, quality);

    let ys_block_matrix = block::split_to_block_matrix(&ys);
    let cbs_block_matrix = block::split_to_block_matrix(&cbs);
    let crs_block_matrix = block::split_to_block_matrix(&crs);

    let ys_quantized = ys_block_matrix.apply_quantization(&scaled_luminance_quant_table);
    let cbs_quantized = cbs_block_matrix.apply_quantization(&scaled_chrominance_quant_table);
    let crs_quantized = crs_block_matrix.apply_quantization(&scaled_chrominance_quant_table);

    // Create temporary canvas1 so that we can draw scaled image to proper canvas
    let tmp_canvas = web_sys::window()
        .unwrap()
        .document()
        .unwrap()
        .create_element("canvas")
        .unwrap()
        .dyn_into::<web_sys::HtmlCanvasElement>()
        .unwrap();
    tmp_canvas.set_height(BLOCK_SIZE);
    tmp_canvas.set_width(BLOCK_SIZE);
    let tmp_ctx = canvas_context_2d(&tmp_canvas);

    draw_spatial_channel(
        &ys_quantized,
        ys_block_matrix.width,
        ys_block_matrix.height,
        &ctx_ys,
        &tmp_canvas,
        &tmp_ctx,
    );
    draw_spatial_channel(
        &cbs_quantized,
        cbs_block_matrix.width,
        cbs_block_matrix.height,
        &ctx_cbs,
        &tmp_canvas,
        &tmp_ctx,
    );
    draw_spatial_channel(
        &crs_quantized,
        crs_block_matrix.width,
        crs_block_matrix.height,
        &ctx_crs,
        &tmp_canvas,
        &tmp_ctx,
    );
}

#[allow(clippy::ptr_arg)]
fn draw_spatial_channel(
    data: &Vec<[[u8; 8]; 8]>,
    width: usize,
    height: usize,
    canvas_context: &web_sys::CanvasRenderingContext2d,
    tmp_canvas: &web_sys::HtmlCanvasElement,
    tmp_ctx: &web_sys::CanvasRenderingContext2d,
) {
    let mut image_data = vec![0; (BLOCK_SIZE * BLOCK_SIZE * 4) as usize];

    for v in 0..width {
        for u in 0..height {
            let spatial = data[u + v * width];
            write_to_image_data(&mut image_data, &spatial, u, v);
        }
    }

    let image_data = web_sys::ImageData::new_with_u8_clamped_array(
        wasm_bindgen::Clamped(&image_data),
        BLOCK_SIZE,
    )
    .unwrap();

    tmp_ctx.put_image_data(&image_data, 0.0, 0.0).unwrap();
    canvas_context.scale(ZOOM as f64, ZOOM as f64).unwrap();
    canvas_context
        .draw_image_with_html_canvas_element(tmp_canvas, 0.0, 0.0)
        .unwrap();
    canvas_context
        .scale(1.0 / ZOOM as f64, 1.0 / ZOOM as f64)
        .unwrap();
}

fn write_to_image_data(image_data: &mut Vec<u8>, spatial: &[[u8; 8]; 8], u: usize, v: usize) {
    for y in 0..8 {
        for x in 0..8 {
            let offset = ((v * 8 + y) * BLOCK_SIZE as usize + (u * 8) + x) * 4;
            image_data[offset] = 255 - spatial[y][x];
            image_data[offset + 1] = 255 - spatial[y][x];
            image_data[offset + 2] = 255 - spatial[y][x];
            image_data[offset + 3] = 255;
        }
    }
}

struct_urls!();
#[allow(dead_code)]
impl<'a> Urls<'a> {
    pub fn base(self) -> Url {
        self.base_url()
    }
}

// ------ ------
//    Update
// ------ ------

pub(crate) fn update(msg: Msg, model: &mut Model, orders: &mut impl Orders<Msg>) {
    match msg {
        Msg::FileChooserLoadImage(file) => {
            let file_blob = gloo_file::Blob::from(file);
            orders.perform_cmd(async move {
                let raw_image = utils::load_image(file_blob).await;
                Msg::ImageLoaded(raw_image)
            });
            model.quality = 50;
            model.state = State::PreImageView
        }
        Msg::FileChooserDragStarted => model.file_chooser_zone_active = true,
        Msg::FileChooserDragLeave => model.file_chooser_zone_active = false,
        Msg::ImageLoaded(raw_image) => {
            draw_original_image_preview(&model.original_canvas_preview, &raw_image);
            draw_original_image(&model.original_canvas, &raw_image);

            let raw_image_rc = Rc::new(raw_image);
            let image_window =
                RawImageWindow::new(raw_image_rc.clone(), 0, 0, BLOCK_SIZE, BLOCK_SIZE);
            let ycbcr = image_window.to_rgb_image().to_ycbcr_image();
            draw_ycbcr(
                &model.ys_canvas,
                &model.cbs_canvas,
                &model.crs_canvas,
                &ycbcr,
            );
            draw_dct_quantized(
                &model.ys_quant_canvas,
                &model.cbs_quant_canvas,
                &model.crs_quant_canvas,
                &ycbcr,
                50,
            );
            model.state = State::ImageView(ImagePack {
                raw_image: raw_image_rc.clone(),
                image_window,
                start_x: 0,
                start_y: 0,
                ycbcr,
            });
        }
        Msg::QualityUpdated(quality) => {
            if let State::ImageView(pack) = &model.state {
                model.quality = quality;
                draw_dct_quantized(
                    &model.ys_quant_canvas,
                    &model.cbs_quant_canvas,
                    &model.crs_quant_canvas,
                    &pack.ycbcr,
                    quality,
                );
            }
        }
        Msg::PreviewCanvasClicked(x, y) => {
            if let State::ImageView(ref mut pack) = model.state {
                let canvas_rect = &model
                    .original_canvas_preview
                    .get()
                    .unwrap()
                    .get_bounding_client_rect();
                let canvas_x = canvas_rect.left();
                let canvas_y = canvas_rect.top();

                let image_click_x: i32 = ((x - canvas_x as i32) as u32 * pack.raw_image.width()
                    / (BLOCK_SIZE * ZOOM)) as i32;
                let image_click_y: i32 = ((y - canvas_y as i32) as u32 * pack.raw_image.height()
                    / (BLOCK_SIZE * ZOOM)) as i32;

                let image_x: u32 = cmp::min(
                    cmp::max(image_click_x - (BLOCK_SIZE / 2) as i32, 0),
                    (pack.raw_image.width() - BLOCK_SIZE) as i32,
                ) as u32;
                let image_y: u32 = cmp::min(
                    cmp::max(image_click_y - (BLOCK_SIZE / 2) as i32, 0),
                    (pack.raw_image.height() - BLOCK_SIZE) as i32,
                ) as u32;

                let start_x: u32 = image_x * ZOOM;
                let start_y: u32 = image_y * ZOOM;

                pack.image_window.start_x = image_x;
                pack.image_window.start_y = image_y;

                pack.ycbcr = pack.image_window.to_rgb_image().to_ycbcr_image();
                draw_ycbcr(
                    &model.ys_canvas,
                    &model.cbs_canvas,
                    &model.crs_canvas,
                    &pack.ycbcr,
                );
                draw_dct_quantized(
                    &model.ys_quant_canvas,
                    &model.cbs_quant_canvas,
                    &model.crs_quant_canvas,
                    &pack.ycbcr,
                    model.quality,
                );

                &model
                    .original_canvas_scrollable_div_wrapper
                    .get()
                    .unwrap()
                    .scroll_to_with_x_and_y(start_x.into(), start_y.into());
            }
        }
        Msg::BlockChosen(x, y) => {
            if let State::ImageView(ref mut pack) = model.state {
                let canvas_rect = &model
                    .original_canvas
                    .get()
                    .unwrap()
                    .get_bounding_client_rect();
                let canvas_x = canvas_rect.left();
                let canvas_y = canvas_rect.top();

                let start_x: u32 = ((x - canvas_x as i32) as u32 / (16 * ZOOM)) * 16;
                let start_y: u32 = ((y - canvas_y as i32) as u32 / (16 * ZOOM)) * 16;

                draw_block_choice_indicator(
                    &model.original_canvas,
                    &pack.raw_image,
                    start_x,
                    start_y,
                );
            }
        }
    }
}
