use seed::prelude::*;
use seed::*;
use std::cmp;
use strum::IntoEnumIterator;

use super::model::*;
use super::utils;
use super::view::*;
use crate::image::pixel::RGB;
use crate::image::RawImageWindow;
use crate::{
    block::{self, Block, BlockMatrix},
    image, quant, Msg as GMsg, BLOCK_SIZE, ZOOM,
};
use std::rc::Rc;

use std::collections::HashMap;
use web_sys::HtmlCanvasElement;
use web_sys::HtmlDivElement;

pub fn init(url: Url) -> Option<Model> {
    let base_url = url.to_base_url();

    let mut canvas_map = HashMap::<CanvasName, ElRef<HtmlCanvasElement>>::new();
    for canvas_name in CanvasName::iter() {
        canvas_map.insert(canvas_name, ElRef::<HtmlCanvasElement>::default());
    }

    Some(Model {
        file_chooser_zone_active: false,
        base_url,
        state: State::FileChooser,
        canvas_map,
        original_canvas_scrollable_div_wrapper: ElRef::<HtmlDivElement>::default(),
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

    // Adjust original image preview width so that it isn't squeezed
    let new_width =
        ((image.width() as f64 / image.height() as f64) * canvas.height() as f64) as u32;
    canvas.set_width(new_width);

    // Set scale and draw scaled image from temporary canvas1
    let ctx = canvas_context_2d(&canvas);
    ctx.scale(
        canvas.width() as f64 / tmp_canvas.width() as f64,
        canvas.height() as f64 / tmp_canvas.height() as f64,
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

    ctx.scale(ZOOM as f64, ZOOM as f64).unwrap();
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
    canvas_map: &HashMap<CanvasName, ElRef<HtmlCanvasElement>>,
    image: &image::YCbCrImage,
) {
    let canvas_ys = canvas_map.get(&CanvasName::Ys).unwrap();
    let ctx_ys = canvas_context_2d(&canvas_ys.get().unwrap());
    let canvas_cbs = canvas_map.get(&CanvasName::Cbs).unwrap();
    let ctx_cbs = canvas_context_2d(&canvas_cbs.get().unwrap());
    let canvas_crs = canvas_map.get(&CanvasName::Crs).unwrap();
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
    ctx_ys.scale(1.0 / ZOOM as f64, 1.0 / ZOOM as f64).unwrap();

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
    ctx_cbs.scale(1.0 / ZOOM as f64, 1.0 / ZOOM as f64).unwrap();

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
    ctx_crs.scale(1.0 / ZOOM as f64, 1.0 / ZOOM as f64).unwrap();
}

fn draw_dct_quantized(
    canvas_map: &HashMap<CanvasName, ElRef<HtmlCanvasElement>>,
    image: &image::YCbCrImage,
    image_window: &RawImageWindow,
    quality: u8,
) {
    let canvas_ys_quant = canvas_map.get(&CanvasName::YsQuant).unwrap();
    let ctx_ys = canvas_context_2d(&canvas_ys_quant.get().unwrap());
    let canvas_cbs_quant = canvas_map.get(&CanvasName::CbsQuant).unwrap();
    let ctx_cbs = canvas_context_2d(&canvas_cbs_quant.get().unwrap());
    let canvas_crs_quant = canvas_map.get(&CanvasName::CrsQuant).unwrap();
    let ctx_crs = canvas_context_2d(&canvas_crs_quant.get().unwrap());

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
        &ys_quantized.blocks,
        ys_block_matrix.width,
        ys_block_matrix.height,
        &ctx_ys,
        &tmp_canvas,
        &tmp_ctx,
    );
    draw_spatial_channel(
        &cbs_quantized.blocks,
        cbs_block_matrix.width,
        cbs_block_matrix.height,
        &ctx_cbs,
        &tmp_canvas,
        &tmp_ctx,
    );
    draw_spatial_channel(
        &crs_quantized.blocks,
        crs_block_matrix.width,
        crs_block_matrix.height,
        &ctx_crs,
        &tmp_canvas,
        &tmp_ctx,
    );

    draw_ycbcr_recovered(
        &canvas_map,
        &ys_quantized,
        &cbs_quantized,
        &crs_quantized,
        &image_window,
        quality,
    );
}

#[allow(clippy::ptr_arg)]
fn draw_spatial_channel(
    data: &Vec<Block>,
    width: usize,
    height: usize,
    canvas_context: &web_sys::CanvasRenderingContext2d,
    tmp_canvas: &web_sys::HtmlCanvasElement,
    tmp_ctx: &web_sys::CanvasRenderingContext2d,
) {
    let mut image_data = vec![0; (BLOCK_SIZE * BLOCK_SIZE * 4) as usize];

    for v in 0..width {
        for u in 0..height {
            let spatial = &data[u + v * width];
            write_to_image_data(&mut image_data, &spatial.0, u, v);
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

fn draw_ycbcr_recovered(
    canvas_map: &HashMap<CanvasName, ElRef<HtmlCanvasElement>>,
    ys_quantized: &BlockMatrix,
    cbs_quantized: &BlockMatrix,
    crs_quantized: &BlockMatrix,
    image_window: &RawImageWindow,
    quality: u8,
) {
    let canvas_ys_recovered = canvas_map.get(&CanvasName::YsRecovered).unwrap();
    let ctx_ys = canvas_context_2d(&canvas_ys_recovered.get().unwrap());
    let canvas_cbs_recovered = canvas_map.get(&CanvasName::CbsRecovered).unwrap();
    let ctx_cbs = canvas_context_2d(&canvas_cbs_recovered.get().unwrap());
    let canvas_crs_recovered = canvas_map.get(&CanvasName::CrsRecovered).unwrap();
    let ctx_crs = canvas_context_2d(&canvas_crs_recovered.get().unwrap());

    let scaled_luminance_quant_table =
        quant::scale_quantization_table(&quant::LUMINANCE_QUANTIZATION_TABLE, quality);
    let scaled_chrominance_quant_table =
        quant::scale_quantization_table(&quant::CHROMINANCE_QUANTIZATION_TABLE, quality);

    let ys_dequantized = ys_quantized.undo_quantization(&scaled_luminance_quant_table);
    let cbs_dequantized = cbs_quantized.undo_quantization(&scaled_chrominance_quant_table);
    let crs_dequantized = crs_quantized.undo_quantization(&scaled_chrominance_quant_table);

    let ys = ys_dequantized.flatten();
    let cbs = cbs_dequantized.flatten();
    let crs = crs_dequantized.flatten();

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

    let ys_image_data =
        web_sys::ImageData::new_with_u8_clamped_array(wasm_bindgen::Clamped(&ys_image), BLOCK_SIZE)
            .unwrap();
    tmp_ctx.put_image_data(&ys_image_data, 0.0, 0.0).unwrap();
    ctx_ys.scale(ZOOM as f64, ZOOM as f64).unwrap();
    ctx_ys
        .draw_image_with_html_canvas_element(&tmp_canvas, 0.0, 0.0)
        .unwrap();
    ctx_ys.scale(1.0 / ZOOM as f64, 1.0 / ZOOM as f64).unwrap();

    let cbs_image_data = web_sys::ImageData::new_with_u8_clamped_array(
        wasm_bindgen::Clamped(&cbs_image),
        BLOCK_SIZE,
    )
    .unwrap();
    tmp_ctx.put_image_data(&cbs_image_data, 0.0, 0.0).unwrap();
    ctx_cbs.scale(ZOOM as f64, ZOOM as f64).unwrap();
    ctx_cbs
        .draw_image_with_html_canvas_element(&tmp_canvas, 0.0, 0.0)
        .unwrap();
    ctx_cbs.scale(1.0 / ZOOM as f64, 1.0 / ZOOM as f64).unwrap();

    let crs_image_data = web_sys::ImageData::new_with_u8_clamped_array(
        wasm_bindgen::Clamped(&crs_image),
        BLOCK_SIZE,
    )
    .unwrap();
    tmp_ctx.put_image_data(&crs_image_data, 0.0, 0.0).unwrap();
    ctx_crs.scale(ZOOM as f64, ZOOM as f64).unwrap();
    ctx_crs
        .draw_image_with_html_canvas_element(&tmp_canvas, 0.0, 0.0)
        .unwrap();
    ctx_crs.scale(1.0 / ZOOM as f64, 1.0 / ZOOM as f64).unwrap();

    draw_image_recovered(
        canvas_map,
        ys,
        cbs,
        crs,
        &image_window,
    );
}

fn draw_image_recovered(
    canvas_map: &HashMap<CanvasName, ElRef<HtmlCanvasElement>>,
    ys: Vec<u8>,
    cbs: Vec<u8>,
    crs: Vec<u8>,
    image_window: &RawImageWindow
) {
    let image_preview_for_comparison_canvas = canvas_map.get(&CanvasName::ImagePreviewForComparison).unwrap();
    let ctx = canvas_context_2d(&image_preview_for_comparison_canvas.get().unwrap());
    let image_data = &image_window.to_rgb_image().to_image_data();
    let image =
        web_sys::ImageData::new_with_u8_clamped_array(wasm_bindgen::Clamped(&image_data), BLOCK_SIZE)
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
    tmp_canvas.set_height(BLOCK_SIZE);
    tmp_canvas.set_width(BLOCK_SIZE);
    let tmp_ctx = canvas_context_2d(&tmp_canvas);
    tmp_ctx.put_image_data(&image, 0.0, 0.0).unwrap();
    ctx.scale(ZOOM as f64, ZOOM as f64).unwrap();
    ctx.draw_image_with_html_canvas_element(&tmp_canvas, 0.0, 0.0)
        .unwrap();
    ctx.scale(1.0 / ZOOM as f64, 1.0 / ZOOM as f64).unwrap();

    let image_recovered_canvas = canvas_map.get(&CanvasName::ImageRecovered).unwrap();
    let ctx = canvas_context_2d(&image_recovered_canvas.get().unwrap());

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

    let image = ys
        .iter()
        .zip(cbs.iter())
        .zip(crs.iter())
        .flat_map(|((y, cb), cr)| {
            let RGB { r, g, b } = image::pixel::YCbCr {
                y: *y,
                cb: *cb,
                cr: *cr,
            }
            .to_rgb();
            vec![r, g, b, 255]
        })
        .collect::<Vec<u8>>();

    let image =
        web_sys::ImageData::new_with_u8_clamped_array(wasm_bindgen::Clamped(&image), BLOCK_SIZE)
            .unwrap();
    tmp_ctx.put_image_data(&image, 0.0, 0.0).unwrap();
    ctx.scale(ZOOM as f64, ZOOM as f64).unwrap();
    ctx.draw_image_with_html_canvas_element(&tmp_canvas, 0.0, 0.0)
        .unwrap();
    ctx.scale(1.0 / ZOOM as f64, 1.0 / ZOOM as f64).unwrap();
}

fn write_to_image_data(image_data: &mut Vec<u8>, spatial: &[[i16; 8]; 8], u: usize, v: usize) {
    for y in 0..8 {
        for x in 0..8 {
            let offset = ((v * 8 + y) * BLOCK_SIZE as usize + (u * 8) + x) * 4;
            image_data[offset] = 255 - spatial[y][x].abs().clamp(0, 255) as u8;
            image_data[offset + 1] = 255 - spatial[y][x].abs().clamp(0, 255) as u8;
            image_data[offset + 2] = 255 - spatial[y][x].abs().clamp(0, 255) as u8;
            image_data[offset + 3] = 255;
        }
    }
}

fn turn_antialiasing_off(canvas_map: &HashMap<CanvasName, ElRef<HtmlCanvasElement>>) {
    for (_canvas_name, canvas) in canvas_map {
        turn_antialising_off_for_specific_canvas(canvas)
    }

    fn turn_antialising_off_for_specific_canvas(canvas: &ElRef<HtmlCanvasElement>) {
        let ctx = canvas_context_2d(&canvas.get().unwrap());
        ctx.set_image_smoothing_enabled(false);
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
            turn_antialiasing_off(&model.canvas_map);
            draw_original_image_preview(
                &model.canvas_map.get(&CanvasName::OriginalPreview).unwrap(),
                &raw_image,
            );
            draw_original_image(
                &model.canvas_map.get(&CanvasName::Original).unwrap(),
                &raw_image,
            );

            let raw_image_rc = Rc::new(raw_image);
            let image_window =
                RawImageWindow::new(raw_image_rc.clone(), 0, 0, BLOCK_SIZE, BLOCK_SIZE);
            let ycbcr = image_window.to_rgb_image().to_ycbcr_image();
            draw_ycbcr(&model.canvas_map, &ycbcr);
            draw_dct_quantized(&model.canvas_map, &ycbcr, &image_window, 50);
            model.state = State::ImageView(ImagePack {
                raw_image: raw_image_rc,
                image_window,
                start_x: 0,
                start_y: 0,
                ycbcr,
            });
        }
        Msg::QualityUpdated(quality) => {
            if let State::ImageView(pack) = &model.state {
                model.quality = quality;
                draw_dct_quantized(&model.canvas_map, &pack.ycbcr, &pack.image_window, quality);
            }
        }
        Msg::PreviewCanvasClicked(x, y) => {
            if let State::ImageView(ref mut pack) = model.state {
                let canvas_rect = &model
                    .canvas_map
                    .get(&CanvasName::OriginalPreview)
                    .unwrap()
                    .get()
                    .unwrap()
                    .get_bounding_client_rect();

                let canvas_x = canvas_rect.left();
                let canvas_y = canvas_rect.top();

                let image_click_x: i32 = ((x - canvas_x as i32) as u32 * pack.raw_image.height()
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
                draw_ycbcr(&model.canvas_map, &pack.ycbcr);
                draw_dct_quantized(&model.canvas_map, &pack.ycbcr, &pack.image_window, model.quality);

                model
                    .original_canvas_scrollable_div_wrapper
                    .get()
                    .unwrap()
                    .scroll_to_with_x_and_y(start_x.into(), start_y.into());
            }
        }
        Msg::BlockChosen(x, y) => {
            if let State::ImageView(ref mut pack) = model.state {
                let canvas_rect = &model
                    .canvas_map
                    .get(&CanvasName::Original)
                    .unwrap()
                    .get()
                    .unwrap()
                    .get_bounding_client_rect();
                let canvas_x = canvas_rect.left();
                let canvas_y = canvas_rect.top();

                let start_x: u32 = ((x - canvas_x as i32) as u32 / (16 * ZOOM)) * 16;
                let start_y: u32 = ((y - canvas_y as i32) as u32 / (16 * ZOOM)) * 16;

                draw_block_choice_indicator(
                    &model.canvas_map.get(&CanvasName::Original).unwrap(),
                    &pack.raw_image,
                    start_x,
                    start_y,
                );
            }
        }
    }
}
