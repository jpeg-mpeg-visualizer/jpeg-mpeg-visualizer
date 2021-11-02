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
        original_image_canvas: ElRef::<HtmlCanvasElement>::default(),
        canvas_map,
        preview_canvas_map,
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

fn draw_block_choice_indicator_for_canvas(
    canvas: &ElRef<HtmlCanvasElement>,
    start_x: f64,
    start_y: f64,
) {
    let ctx = canvas_context_2d(&canvas.get().unwrap());
    // Draw rect
    ctx.begin_path();
    let line_width: f64 = 2.0;
    ctx.set_line_width(line_width);
    ctx.stroke_rect(
        start_x - line_width / 2.0,
        start_y - line_width / 2.0,
        8.0 * ZOOM as f64 + line_width / 2.0,
        8.0 * ZOOM as f64 + line_width / 2.0,
    );
}
fn draw_block_choice_indicators_for_ordinary(
    canvas_map: &HashMap<CanvasName, ElRef<HtmlCanvasElement>>,
    start_x: f64,
    start_y: f64,
) {
    for (_canvas_name, canvas) in canvas_map {
        draw_block_choice_indicator_for_canvas(&canvas, start_x, start_y);
    }
}
fn draw_block_choice_indicators_for_preview(
    preview_canvas_map: &HashMap<PreviewCanvasName, ElRef<HtmlCanvasElement>>,
    start_x: f64,
    start_y: f64,
) {
    for (_canvas_name, canvas) in preview_canvas_map {
        draw_block_choice_indicator_for_canvas(&canvas, start_x, start_y);
    }
}

fn draw_block_choice_indicators(
    canvas_map: &HashMap<CanvasName, ElRef<HtmlCanvasElement>>,
    preview_canvas_map: &HashMap<PreviewCanvasName, ElRef<HtmlCanvasElement>>,
    start_x: f64,
    start_y: f64,
) {
    draw_block_choice_indicators_for_ordinary(&canvas_map, start_x, start_y);
    draw_block_choice_indicators_for_preview(&preview_canvas_map, start_x, start_y);
}

fn draw_ycbcr(canvas_map: &HashMap<CanvasName, ElRef<HtmlCanvasElement>>, pack: &mut ImagePack) {
    let ys = pack.ycbcr.to_ys_channel();
    let cbs = pack.ycbcr.to_cbs_channel();
    let crs = pack.ycbcr.to_crs_channel();

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

    draw_and_fill_content_map(&canvas_map, CanvasName::Ys, pack, ys_image);
    draw_and_fill_content_map(&canvas_map, CanvasName::Cbs, pack, cbs_image);
    draw_and_fill_content_map(&canvas_map, CanvasName::Crs, pack, crs_image);
}

fn draw_dct_quantized(
    canvas_map: &HashMap<CanvasName, ElRef<HtmlCanvasElement>>,
    pack: &mut ImagePack,
    quality: u8,
) {
    let ys = pack.ycbcr.to_ys_channel();
    let cbs = pack.ycbcr.to_cbs_channel();
    let crs = pack.ycbcr.to_crs_channel();

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
        canvas_map,
        CanvasName::YsQuant,
        pack,
    );
    draw_spatial_channel(
        &cbs_quantized.blocks,
        cbs_block_matrix.width,
        cbs_block_matrix.height,
        canvas_map,
        CanvasName::CbsQuant,
        pack,
    );
    draw_spatial_channel(
        &crs_quantized.blocks,
        crs_block_matrix.width,
        crs_block_matrix.height,
        canvas_map,
        CanvasName::CrsQuant,
        pack,
    );

    draw_ycbcr_recovered(
        &canvas_map,
        &ys_quantized,
        &cbs_quantized,
        &crs_quantized,
        pack,
        quality,
    );
}

#[allow(clippy::ptr_arg)]
fn draw_spatial_channel(
    data: &Vec<Block>,
    width: usize,
    height: usize,
    canvas_map: &HashMap<CanvasName, ElRef<HtmlCanvasElement>>,
    canvas_name: CanvasName,
    pack: &mut ImagePack,
) {
    let mut image_data = vec![0; (BLOCK_SIZE * BLOCK_SIZE * 4) as usize];

    for v in 0..width {
        for u in 0..height {
            let spatial = &data[u + v * width];
            write_to_image_data(&mut image_data, &spatial.0, u, v);
        }
    }
    draw_and_fill_content_map(&canvas_map, canvas_name, pack, image_data);
}

fn draw_ycbcr_recovered(
    canvas_map: &HashMap<CanvasName, ElRef<HtmlCanvasElement>>,
    ys_quantized: &BlockMatrix,
    cbs_quantized: &BlockMatrix,
    crs_quantized: &BlockMatrix,
    pack: &mut ImagePack,
    quality: u8,
) {
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

    draw_and_fill_content_map(&canvas_map, CanvasName::YsRecovered, pack, ys_image);
    draw_and_fill_content_map(&canvas_map, CanvasName::CbsRecovered, pack, cbs_image);
    draw_and_fill_content_map(&canvas_map, CanvasName::CrsRecovered, pack, crs_image);

    draw_image_recovered(canvas_map, ys, cbs, crs, pack);
}

fn draw_image_recovered(
    canvas_map: &HashMap<CanvasName, ElRef<HtmlCanvasElement>>,
    ys: Vec<u8>,
    cbs: Vec<u8>,
    crs: Vec<u8>,
    pack: &mut ImagePack,
) {
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
    let input_image = &pack.image_window.to_rgb_image().to_image();
    let image_diff = get_image_diff(&output_image, &input_image);
    draw_and_fill_content_map(&canvas_map, CanvasName::ImageRecovered, pack, output_image);

    // TODO: Consider optimizing input_image calculation -> instead of to_rgb and then to image, make one method that would do both but in less steps!
    draw_and_fill_content_map(&canvas_map, CanvasName::Difference, pack, image_diff);
}

fn draw_and_fill_content_map(
    canvas_map: &HashMap<CanvasName, ElRef<HtmlCanvasElement>>,
    canvas_name: CanvasName,
    pack: &mut ImagePack,
    image_data: Vec<u8>,
) {
    let canvas = canvas_map.get(&canvas_name).unwrap();
    draw_scaled_image_default(&canvas, &image_data);
    pack.canvases_content.insert(canvas_name, image_data);
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
    preview_canvas_map: &HashMap<PreviewCanvasName, ElRef<HtmlCanvasElement>>,
) {
    for (_canvas_name, canvas) in canvas_map {
        turn_antialising_off_for_specific_canvas(&canvas);
    }
    for (_canvas_name, canvas) in preview_canvas_map {
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
fn redraw_ordinary_canvases(
    canvas_map: &HashMap<CanvasName, ElRef<HtmlCanvasElement>>,
    canvases_content: &HashMap<CanvasName, Vec<u8>>,
) {
    for (canvas_name, image) in canvases_content {
        let canvas = canvas_map.get(canvas_name).unwrap();
        draw_scaled_image_default(&canvas, &image);
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
            draw_original_image_preview(&model.original_image_canvas, &raw_image);

            let raw_image_rc = Rc::new(raw_image);
            // TODO: Consider drawing initial rect here (currently it is drawn only after user chooses a block - but that might be what we actually want)
            let image_window =
                RawImageWindow::new(raw_image_rc.clone(), 0, 0, BLOCK_SIZE, BLOCK_SIZE);
            draw_input_previews(&model.preview_canvas_map, &image_window);
            let ycbcr = image_window.to_rgb_image().to_ycbcr_image();
            let canvases_content: HashMap<CanvasName, Vec<u8>> =
                HashMap::<CanvasName, Vec<u8>>::new();
            let mut pack: ImagePack = ImagePack {
                raw_image: raw_image_rc,
                image_window,
                ycbcr,
                // -1.0 meaning nothing is chosen
                chosen_block_x: -1.0,
                chosen_block_y: -1.0,
                canvases_content,
            };
            draw_ycbcr(&model.canvas_map, &mut pack);
            draw_dct_quantized(&model.canvas_map, &mut pack, 50);
            model.state = State::ImageView(pack);
        }
        Msg::QualityUpdated(quality) => {
            if let State::ImageView(ref mut pack) = model.state {
                model.quality = quality;
                draw_dct_quantized(&model.canvas_map, pack, quality);
                draw_block_choice_indicators_for_ordinary(
                    &model.canvas_map,
                    pack.chosen_block_x,
                    pack.chosen_block_y,
                );
            }
        }
        Msg::PreviewCanvasClicked(x, y) => {
            if let State::ImageView(ref mut pack) = model.state {
                let preview_canvas_ref = &model.original_image_canvas
                let preview_canvas = preview_canvas_ref.get().unwrap();
                let canvas_rect = preview_canvas.get_bounding_client_rect();

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

                pack.image_window.start_x = image_x;
                pack.image_window.start_y = image_y;
                pack.chosen_block_x = -1.0;
                pack.chosen_block_y = -1.0;

                pack.ycbcr = pack.image_window.to_rgb_image().to_ycbcr_image();

                draw_input_previews(&model.preview_canvas_map, &pack.image_window);
                draw_ycbcr(&model.canvas_map, pack);
                draw_dct_quantized(&model.canvas_map, pack, model.quality);

                // We need to calculate offset of line_width so that all pixels of the image window are inside stroked rect (as in not covered by the lines)
                let line_width: i32 = 4;
                draw_original_image_preview(&preview_canvas_ref, &pack.raw_image);
                let preview_ctx = canvas_context_2d(&preview_canvas);
                let rect_a = (((preview_canvas.height() * BLOCK_SIZE) / pack.raw_image.height())
                    as i32
                    + line_width) as f64;
                let rect_x = cmp::max(
                    0,
                    ((image_x * preview_canvas.height()) / pack.raw_image.height()) as i32
                        - line_width / 2,
                ) as f64;
                let rect_y = cmp::max(
                    0,
                    ((image_y * preview_canvas.width()) / pack.raw_image.width()) as i32
                        - line_width / 2,
                ) as f64;

                preview_ctx.set_line_width(line_width as f64);
                preview_ctx.stroke_rect(rect_x, rect_y, rect_a, rect_a);
            }
        }
        Msg::BlockChosen(x, y, rect_x, rect_y) => {
            if let State::ImageView(ref mut pack) = model.state {
                let start_x: f64 = cmp::min(
                    (x - rect_x) - (x - rect_x) % (8 * ZOOM as i32),
                    ((BLOCK_SIZE - 8) * ZOOM) as i32,
                ) as f64;
                let start_y: f64 = cmp::min(
                    (y - rect_y) - (y - rect_y) % (8 * ZOOM as i32),
                    ((BLOCK_SIZE - 8) * ZOOM) as i32,
                ) as f64;

                // Reset previous block choice indicators
                redraw_ordinary_canvases(&model.canvas_map, &pack.canvases_content);
                draw_input_previews(&model.preview_canvas_map, &pack.image_window);
                // Draw actual square (block choice indicator)
                draw_block_choice_indicators(
                    &model.canvas_map,
                    &model.preview_canvas_map,
                    start_x,
                    start_y,
                );
                pack.chosen_block_x = start_x;
                pack.chosen_block_y = start_y;
            }
        }
    }
}
