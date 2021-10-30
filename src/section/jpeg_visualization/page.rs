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

use super::drawing_utils::{
    draw_scaled_image_default, draw_scaled_image_with_image_data_with_w_h_and_scale,
};
use super::utils::get_image_diff;
use std::collections::HashMap;
use web_sys::HtmlCanvasElement;
use web_sys::HtmlDivElement;

pub fn init(url: Url) -> Option<Model> {
    let base_url = url.to_base_url();

    let mut canvas_map = HashMap::<CanvasName, ElRef<HtmlCanvasElement>>::new();
    for canvas_name in CanvasName::iter() {
        canvas_map.insert(canvas_name, ElRef::<HtmlCanvasElement>::default());
    }
    let mut preview_canvas_map = HashMap::<PreviewCanvasName, ElRef<HtmlCanvasElement>>::new();
    for canvas_name in PreviewCanvasName::iter() {
        preview_canvas_map.insert(canvas_name, ElRef::<HtmlCanvasElement>::default());
    }

    Some(Model {
        file_chooser_zone_active: false,
        base_url,
        state: State::FileChooser,
        canvas_map,
        preview_canvas_map,
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

    // Adjust original image preview width so that it isn't squeezed
    let new_width =
        ((image.width() as f64 / image.height() as f64) * canvas.height() as f64) as u32;
    canvas.set_width(new_width);

    draw_scaled_image_with_image_data_with_w_h_and_scale(
        &original_canvas_preview,
        &img,
        image.width(),
        image.height(),
        canvas.width() as f64 / image.width() as f64,
        canvas.height() as f64 / image.height() as f64,
    );
}

fn draw_original_image(original_image_canvas: &ElRef<HtmlCanvasElement>, image: &image::RawImage) {
    let canvas = original_image_canvas.get().unwrap();
    canvas.set_height(image.height() * ZOOM);
    canvas.set_width(image.width() * ZOOM);
    let img = web_sys::ImageData::new_with_u8_clamped_array(
        wasm_bindgen::Clamped(image.as_ref()),
        image.width(),
    )
    .unwrap();

    draw_scaled_image_with_image_data_with_w_h_and_scale(
        &original_image_canvas,
        &img,
        image.width(),
        image.height(),
        ZOOM as f64,
        ZOOM as f64,
    );
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
    let canvas_cbs = canvas_map.get(&CanvasName::Cbs).unwrap();
    let canvas_crs = canvas_map.get(&CanvasName::Crs).unwrap();

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

    draw_scaled_image_default(&canvas_ys, &ys_image);
    draw_scaled_image_default(&canvas_cbs, &cbs_image);
    draw_scaled_image_default(&canvas_crs, &crs_image);
}

fn draw_dct_quantized(
    canvas_map: &HashMap<CanvasName, ElRef<HtmlCanvasElement>>,
    image: &image::YCbCrImage,
    image_window : &RawImageWindow,
    quality: u8,
) {
    let canvas_ys_quant = canvas_map.get(&CanvasName::YsQuant).unwrap();
    let canvas_cbs_quant = canvas_map.get(&CanvasName::CbsQuant).unwrap();
    let canvas_crs_quant = canvas_map.get(&CanvasName::CrsQuant).unwrap();

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

    draw_spatial_channel(
        &ys_quantized.blocks,
        ys_block_matrix.width,
        ys_block_matrix.height,
        &canvas_ys_quant,
    );
    draw_spatial_channel(
        &cbs_quantized.blocks,
        cbs_block_matrix.width,
        cbs_block_matrix.height,
        &canvas_cbs_quant,
    );
    draw_spatial_channel(
        &crs_quantized.blocks,
        crs_block_matrix.width,
        crs_block_matrix.height,
        &canvas_crs_quant,
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
    canvas: &ElRef<HtmlCanvasElement>,
) {
    let mut image_data = vec![0; (BLOCK_SIZE * BLOCK_SIZE * 4) as usize];

    for v in 0..width {
        for u in 0..height {
            let spatial = &data[u + v * width];
            write_to_image_data(&mut image_data, &spatial.0, u, v);
        }
    }

    draw_scaled_image_default(&canvas, &image_data);
}

fn draw_ycbcr_recovered(
    canvas_map: &HashMap<CanvasName, ElRef<HtmlCanvasElement>>,
    ys_quantized: &BlockMatrix,
    cbs_quantized: &BlockMatrix,
    crs_quantized: &BlockMatrix,
    image_window : &RawImageWindow,
    quality: u8,
) {
    let canvas_ys_recovered = canvas_map.get(&CanvasName::YsRecovered).unwrap();
    let canvas_cbs_recovered = canvas_map.get(&CanvasName::CbsRecovered).unwrap();
    let canvas_crs_recovered = canvas_map.get(&CanvasName::CrsRecovered).unwrap();

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

    draw_scaled_image_default(&canvas_ys_recovered, &ys_image);
    draw_scaled_image_default(&canvas_cbs_recovered, &cbs_image);
    draw_scaled_image_default(&canvas_crs_recovered, &crs_image);

    draw_image_recovered(canvas_map, ys, cbs, crs, &image_window);
}

fn draw_image_recovered(
    canvas_map: &HashMap<CanvasName, ElRef<HtmlCanvasElement>>,
    ys: Vec<u8>,
    cbs: Vec<u8>,
    crs: Vec<u8>,
    image_window : &RawImageWindow
) {
    let image_recovered_canvas = canvas_map.get(&CanvasName::ImageRecovered).unwrap();
    let output_image = ys
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
    draw_scaled_image_default(&image_recovered_canvas, &output_image);

    // TODO: Consider optimizing input_image calculation -> instead of to_rgb and then to image, make one method that would do both but in less steps!
    let input_image = &image_window.to_rgb_image().to_image();
    let image_diff_canvas = canvas_map.get(&CanvasName::Difference).unwrap();
    let image_diff = get_image_diff(&output_image, &input_image);
    log(&image_diff);
    draw_scaled_image_default(&image_diff_canvas, &image_diff);
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

fn turn_antialiasing_off(
    canvas_map: &HashMap<CanvasName, ElRef<HtmlCanvasElement>>,
    preview_canvas_map: &HashMap<PreviewCanvasName, ElRef<HtmlCanvasElement>>
) {
    for (_canvas_name, canvas) in canvas_map {
        turn_antialising_off_for_specific_canvas(&canvas);
    }
    for(_canvas_name, canvas) in preview_canvas_map {
        turn_antialising_off_for_specific_canvas(&canvas);
    }

    fn turn_antialising_off_for_specific_canvas(canvas: &ElRef<HtmlCanvasElement>) {
        let ctx = canvas_context_2d(&canvas.get().unwrap());
        ctx.set_image_smoothing_enabled(false);
    }
}

fn draw_input_previews(
    preview_canvas_map: &HashMap<PreviewCanvasName, ElRef<HtmlCanvasElement>>,
    image_window: &RawImageWindow,
) {
    // TODO: Consider optimizing input_image calculation -> instead of to_rgb and then to image, make one method that would do both but in less steps!
    let input_image = &image_window.to_rgb_image().to_image();
    for (_canvas_name, canvas) in preview_canvas_map {
        draw_scaled_image_default(&canvas, &input_image);
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
            turn_antialiasing_off(&model.canvas_map, &model.preview_canvas_map);
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
            draw_input_previews(&model.preview_canvas_map, &image_window);
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

                draw_input_previews(&model.preview_canvas_map, &pack.image_window);
                draw_ycbcr(&model.canvas_map, &pack.ycbcr);
                draw_dct_quantized(
                    &model.canvas_map,
                    &pack.ycbcr,
                    &pack.image_window,
                    model.quality,
                );

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
