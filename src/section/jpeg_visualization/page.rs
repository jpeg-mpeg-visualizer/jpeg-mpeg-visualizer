use seed::prelude::*;
use seed::*;
use std::cmp;
use strum::IntoEnumIterator;

use plotters::prelude::*;
use plotters_canvas::CanvasBackend;

use super::model::*;
use super::utils;
use super::view::*;
use crate::image::pixel::RGB;
use crate::image::RawImageWindow;
use crate::{
    block::{self, BlockMatrix},
    image, quant, Msg as GMsg, BLOCK_SIZE,
};
use std::rc::Rc;

use super::drawing_utils::{
    draw_scaled_image_default, draw_scaled_image_with_image_data_with_w_h_and_scale,
};
use super::utils::get_image_diff;
use std::collections::HashMap;

use crate::section::jpeg_visualization::utils::{
    create_tmp_canvas, horiz_mult_from_subsampling, vert_mult_from_subsampling,
};
use web_sys::{Blob, HtmlCanvasElement, HtmlImageElement};

pub fn init(url: Url) -> Option<Model> {
    let base_url = url.to_base_url();

    let mut canvas_map = HashMap::<CanvasName, ElRef<HtmlCanvasElement>>::new();
    let mut overlay_map = HashMap::<CanvasName, ElRef<HtmlImageElement>>::new();
    for canvas_name in CanvasName::iter() {
        canvas_map.insert(canvas_name, ElRef::<HtmlCanvasElement>::default());
        overlay_map.insert(canvas_name, ElRef::<HtmlImageElement>::default());
    }
    let mut preview_canvas_map = HashMap::<PreviewCanvasName, ElRef<HtmlCanvasElement>>::new();
    let mut preview_overlay_map = HashMap::<PreviewCanvasName, ElRef<HtmlImageElement>>::new();
    for canvas_name in PreviewCanvasName::iter() {
        preview_canvas_map.insert(canvas_name, ElRef::<HtmlCanvasElement>::default());
        preview_overlay_map.insert(canvas_name, ElRef::<HtmlImageElement>::default());
    }

    let mut plot_map = HashMap::<PlotName, ElRef<HtmlCanvasElement>>::new();
    let mut chosen_block_plot_map = HashMap::<PlotName, ElRef<HtmlCanvasElement>>::new();
    for plot_name in PlotName::iter() {
        plot_map.insert(plot_name, ElRef::<HtmlCanvasElement>::default());
        chosen_block_plot_map.insert(plot_name, ElRef::<HtmlCanvasElement>::default());
    }

    let subsampling_pack = SubsamplingPack { j: 4, a: 4, b: 4 };

    Some(Model {
        file_chooser_zone_active: false,
        base_url,
        state: State::FileChooser,
        original_image_canvas: ElRef::<HtmlCanvasElement>::default(),
        canvas_map,
        preview_canvas_map,
        plot_map,
        chosen_block_plot_map,
        original_image_overlay: ElRef::<HtmlImageElement>::default(),
        overlay_map,
        preview_overlay_map,
        quality: 50,
        zoom: 7,
        is_diff_info_shown: false,
        subsampling_pack,
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

fn prepare_original_image_preview(
    original_canvas_preview: &ElRef<HtmlCanvasElement>,
    original_canvas_overlay: &ElRef<HtmlImageElement>,
    image: &image::RawImage,
) {
    let canvas = original_canvas_preview.get().unwrap();
    // Adjust original image preview width so that it isn't squeezed
    let new_width =
        ((image.width() as f64 / image.height() as f64) * canvas.height() as f64) as u32;
    canvas.set_width(new_width);
    // We also need to adjust the overlay
    original_canvas_overlay.get().unwrap().set_width(new_width);
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

    draw_scaled_image_with_image_data_with_w_h_and_scale(
        &original_canvas_preview,
        &img,
        image.width(),
        image.height(),
        canvas.width() as f64 / image.width() as f64,
        canvas.height() as f64 / image.height() as f64,
    );
}

fn draw_block_choice_indicators(
    overlay_map: &HashMap<CanvasName, ElRef<HtmlImageElement>>,
    preview_overlay_map: &HashMap<PreviewCanvasName, ElRef<HtmlImageElement>>,
    start_x: f64,
    start_y: f64,
    subsampling_pack: &SubsamplingPack,
    zoom: u32,
) {
    let tmp_canvas = create_tmp_canvas();
    tmp_canvas.set_width(BLOCK_SIZE * zoom);
    tmp_canvas.set_height(BLOCK_SIZE * zoom);

    let tmp_ctx = canvas_context_2d(&tmp_canvas);

    tmp_ctx.begin_path();
    // We need to calculate offset of line_width so that all pixels of the image window are inside stroked rect (as in not covered by the lines)
    let line_width: f64 = (2 * zoom / 8) as f64;
    tmp_ctx.set_line_width(line_width);
    tmp_ctx.stroke_rect(
        start_x * zoom as f64 - line_width / 2.0,
        start_y * zoom as f64 - line_width / 2.0,
        8.0 * zoom as f64 + line_width / 2.0,
        8.0 * zoom as f64 + line_width / 2.0,
    );

    let overlay_map_cloned = overlay_map.clone();
    let preview_overlay_map_cloned = preview_overlay_map.clone();
    let f_not_sumbsampled = Closure::once_into_js(move |blob: &Blob| {
        let url = web_sys::Url::create_object_url_with_blob(&blob).unwrap();

        for (canvas_name, overlay_image) in overlay_map_cloned {
            if !is_canvas_subsampled(&canvas_name) {
                overlay_image.get().unwrap().set_src(&url);
            }
        }
        for (_preview_canvas_name, overlay_image) in preview_overlay_map_cloned {
            overlay_image.get().unwrap().set_src(&url);
        }
        // TODO: Consider checking if all images has loaded and after all are loaded revoke no longer needed url
    });
    tmp_canvas
        .to_blob(
            f_not_sumbsampled
                .as_ref()
                .unchecked_ref::<js_sys::Function>(),
        )
        .unwrap();

    let tmp_canvas_for_subsampling = create_tmp_canvas();

    let horiz_mult: usize = horiz_mult_from_subsampling(&subsampling_pack);
    let vert_mult: usize = vert_mult_from_subsampling(&subsampling_pack);

    tmp_canvas_for_subsampling.set_width(BLOCK_SIZE * zoom / horiz_mult as u32);
    tmp_canvas_for_subsampling.set_height(BLOCK_SIZE * zoom / vert_mult as u32);

    let tmp_ctx_for_subsampling = canvas_context_2d(&tmp_canvas_for_subsampling);

    tmp_ctx_for_subsampling.begin_path();
    tmp_ctx_for_subsampling.set_line_width(line_width);
    tmp_ctx_for_subsampling.stroke_rect(
        (start_x - (start_x as u32 % (8 * zoom * horiz_mult as u32)) as f64) / horiz_mult as f64
            - line_width / 2.0,
        (start_y - (start_y as u32 % (8 * zoom * vert_mult as u32)) as f64) / vert_mult as f64
            - line_width / 2.0,
        8.0 * zoom as f64 + line_width / 2.0,
        8.0 * zoom as f64 + line_width / 2.0,
    );
    let overlay_map_cloned_for_subsampling = overlay_map.clone();
    let f_subsampled = Closure::once_into_js(move |blob: &Blob| {
        let url = web_sys::Url::create_object_url_with_blob(&blob).unwrap();

        for (canvas_name, overlay_image) in overlay_map_cloned_for_subsampling {
            if is_canvas_subsampled(&canvas_name) {
                overlay_image.get().unwrap().set_src(&url);
            }
        }
        // TODO: Consider checking if all images has loaded and after all are loaded revoke no longer needed url
    });
    tmp_canvas_for_subsampling
        .to_blob(f_subsampled.as_ref().unchecked_ref::<js_sys::Function>())
        .unwrap();
}

fn draw_ycbcr(
    canvas_map: &HashMap<CanvasName, ElRef<HtmlCanvasElement>>,
    ycbcr: &image::YCbCrImage,
    subsampling_pack: &SubsamplingPack,
    zoom: u32,
) {
    let ys = ycbcr.to_ys_channel();
    let cbs = ycbcr.to_cbs_channel();
    let crs = ycbcr.to_crs_channel();

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

    let horiz_mult: usize = horiz_mult_from_subsampling(&subsampling_pack);
    let vert_mult: usize = vert_mult_from_subsampling(&subsampling_pack);

    assert_eq!(cbs.len(), crs.len());
    let mut cbs_image = Vec::<u8>::new();
    let mut crs_image = Vec::<u8>::new();
    let mut x = 0;
    while x < cbs.len() {
        // force skipping horizontal rows if horiz_mult is more than 1
        if x % (BLOCK_SIZE as usize * horiz_mult) >= BLOCK_SIZE as usize {
            x += BLOCK_SIZE as usize * (horiz_mult - 1);
            continue;
        }
        let mut cbs_avg: usize = 0;
        let mut crs_avg: usize = 0;
        for i in 0..vert_mult {
            for j in 0..horiz_mult {
                let curr = x + i + j * BLOCK_SIZE as usize;
                cbs_avg += cbs[curr] as usize;
                crs_avg += crs[curr] as usize;
            }
        }
        cbs_avg /= vert_mult * horiz_mult;
        crs_avg /= vert_mult * horiz_mult;

        let RGB { r, g, b } = image::pixel::YCbCr {
            y: 128,
            cb: cbs_avg as u8,
            cr: 128,
        }
        .to_rgb();

        cbs_image.push(r);
        cbs_image.push(g);
        cbs_image.push(b);
        cbs_image.push(255);

        let RGB { r, g, b } = image::pixel::YCbCr {
            y: 128,
            cb: 128,
            cr: crs_avg as u8,
        }
        .to_rgb();
        crs_image.push(r);
        crs_image.push(g);
        crs_image.push(b);
        crs_image.push(255);

        x += vert_mult;
    }

    draw_default(&canvas_map, CanvasName::Ys, ys_image, zoom);
    draw_default(&canvas_map, CanvasName::Cbs, cbs_image, zoom);
    draw_default(&canvas_map, CanvasName::Crs, crs_image, zoom);
}

fn draw_dct_quantized(
    canvas_map: &HashMap<CanvasName, ElRef<HtmlCanvasElement>>,
    pack: &mut ImagePack,
    subsampling_pack: &SubsamplingPack,
    quality: u8,
    zoom: u32,
) {
    let ycbcr = &pack.ycbcr;
    let image_window = &pack.image_window;

    let ys = ycbcr.to_ys_channel();
    let cbs = ycbcr
        .to_cbs_channel()
        .into_iter()
        .enumerate()
        .filter(|(index, _x)| subsampling_pack.b != 0 || (*index as u32 / BLOCK_SIZE) % 2 == 0)
        .map(|(_index, x)| x)
        .step_by((subsampling_pack.j / subsampling_pack.a) as usize)
        .collect::<Vec<u8>>();
    let crs = ycbcr
        .to_crs_channel()
        .into_iter()
        .enumerate()
        .filter(|(index, _x)| subsampling_pack.b != 0 || (*index as u32 / BLOCK_SIZE) % 2 == 0)
        .map(|(_index, x)| x)
        .step_by((subsampling_pack.j / subsampling_pack.a) as usize)
        .collect::<Vec<u8>>();

    let scaled_luminance_quant_table =
        quant::scale_quantization_table(&quant::LUMINANCE_QUANTIZATION_TABLE, quality);
    let scaled_chrominance_quant_table =
        quant::scale_quantization_table(&quant::CHROMINANCE_QUANTIZATION_TABLE, quality);

    let height_width_ratio = (subsampling_pack.j / subsampling_pack.a) as f64
        / if subsampling_pack.b == 0 {
            2_f64
        } else {
            1_f64
        };

    let ys_block_matrix = block::split_to_block_matrix(&ys, 1_f64);
    let cbs_block_matrix = block::split_to_block_matrix(&cbs, height_width_ratio);
    let crs_block_matrix = block::split_to_block_matrix(&crs, height_width_ratio);

    let ys_quantized = ys_block_matrix.apply_quantization(&scaled_luminance_quant_table);
    let cbs_quantized = cbs_block_matrix.apply_quantization(&scaled_chrominance_quant_table);
    let crs_quantized = crs_block_matrix.apply_quantization(&scaled_chrominance_quant_table);

    draw_spatial_channel(&ys_quantized, canvas_map, CanvasName::YsQuant, zoom);
    draw_spatial_channel(&cbs_quantized, canvas_map, CanvasName::CbsQuant, zoom);
    draw_spatial_channel(&crs_quantized, canvas_map, CanvasName::CrsQuant, zoom);

    pack.plot_data.insert(PlotName::YsQuant3d, ys_quantized);
    pack.plot_data.insert(PlotName::CbsQuant3d, cbs_quantized);
    pack.plot_data.insert(PlotName::CrsQuant3d, crs_quantized);

    draw_ycbcr_recovered(
        &canvas_map,
        &pack.plot_data.get(&PlotName::YsQuant3d).unwrap(),
        &pack.plot_data.get(&PlotName::CbsQuant3d).unwrap(),
        &pack.plot_data.get(&PlotName::CrsQuant3d).unwrap(),
        &subsampling_pack,
        image_window,
        quality,
        zoom,
    );
}

fn draw_dct_quantized_plots(
    pack: &ImagePack,
    plot_map: &HashMap<PlotName, ElRef<HtmlCanvasElement>>,
    chosen_block_plot_map: &HashMap<PlotName, ElRef<HtmlCanvasElement>>,
    subsampling_pack: &SubsamplingPack,
) {
    let selected_x = pack.chosen_block_x as usize / 8;
    let selected_y = pack.chosen_block_y as usize / 8;

    draw_dct_quantized_plot(
        &pack.plot_data.get(&PlotName::YsQuant3d).unwrap(),
        selected_x,
        selected_y,
        plot_map,
        chosen_block_plot_map,
        PlotName::YsQuant3d,
    );
    let subsampled_x: usize = get_subsampled_block_index_x(selected_x, &subsampling_pack);
    let subsampled_y: usize = get_subsampled_block_index_y(selected_y, &subsampling_pack);
    draw_dct_quantized_plot(
        &pack.plot_data.get(&PlotName::CbsQuant3d).unwrap(),
        subsampled_x,
        subsampled_y,
        plot_map,
        chosen_block_plot_map,
        PlotName::CbsQuant3d,
    );
    draw_dct_quantized_plot(
        &pack.plot_data.get(&PlotName::CrsQuant3d).unwrap(),
        subsampled_x,
        subsampled_y,
        plot_map,
        chosen_block_plot_map,
        PlotName::CrsQuant3d,
    );
}
fn get_subsampled_block_index_x(selected_x: usize, subsampling_pack: &SubsamplingPack) -> usize {
    let horiz_mult: usize = horiz_mult_from_subsampling(&subsampling_pack);
    return selected_x / horiz_mult;
}
fn get_subsampled_block_index_y(selected_y: usize, subsampling_pack: &SubsamplingPack) -> usize {
    let vert_mult: usize = vert_mult_from_subsampling(&subsampling_pack);
    return selected_y / vert_mult;
}

fn draw_dct_quantized_plot(
    data: &BlockMatrix,
    selected_x: usize,
    selected_z: usize,
    canvas_map: &HashMap<PlotName, ElRef<HtmlCanvasElement>>,
    chosen_block_canvas_map: &HashMap<PlotName, ElRef<HtmlCanvasElement>>,
    canvas_name: PlotName,
) {
    let width: usize = data.width;
    let height: usize = data.height;
    let blocks = &data.blocks;

    let canvas = canvas_map.get(&canvas_name).unwrap().get().unwrap();
    let area = CanvasBackend::with_canvas_object(canvas)
        .unwrap()
        .into_drawing_area();

    area.fill(&RGBColor(113, 113, 114)).unwrap();

    let key_points_x = (0..width + 1).map(|x| x as i32 * 8).collect::<Vec<i32>>();
    let key_points_z = (0..height + 1).map(|x| x as i32 * 8).collect::<Vec<i32>>();
    let key_points_y = (0..5).map(|x| x as i32 * 64).collect::<Vec<i32>>();

    let mut chart = ChartBuilder::on(&area)
        .margin(20)
        .build_cartesian_3d(
            (0i32..(width * (8_usize)) as i32).with_key_points(key_points_x),
            (0i32..256i32).with_key_points(key_points_y),
            (0i32..(height * (8_usize)) as i32).with_key_points(key_points_z),
        )
        .unwrap();

    chart.with_projection(|mut pb| {
        pb.pitch = 1.2;
        pb.yaw = 0.5;
        pb.scale = 0.7;
        pb.into_matrix()
    });

    chart.configure_axes().draw().unwrap();

    chart
        .draw_series(
            (0i32..(width * (8usize)) as i32)
                .map(|x| std::iter::repeat(x).zip(0i32..(height * (8usize)) as i32))
                .flatten()
                .map(|(x, z)| {
                    let block_x = (x / 8i32) as usize;
                    let block_z = (z / 8i32) as usize;
                    let block_x_offset = (x % 8i32) as usize;
                    let block_z_offset = (z % 8i32) as usize;
                    let block = blocks[block_x + block_z * width].0;
                    let value = (block[block_z_offset][block_x_offset]).abs().clamp(0, 255) as i32;
                    let face = if value == 0 {
                        TRANSPARENT
                    } else {
                        HSLColor(240.0 / 360.0 - 240.0 / 360.0 * value as f64 / 5.0, 1.0, 0.7)
                            .to_rgba()
                    };

                    let edge = if block_x == selected_x && block_z == selected_z {
                        YELLOW.to_rgba()
                    } else {
                        if value == 0 {
                            TRANSPARENT
                        } else {
                            BLACK.to_rgba()
                        }
                    };
                    Cubiod::new([(x, 0, z), (x + 1, value, z + 1)], face.filled(), &edge)
                }),
        )
        .unwrap();

    // Now chart for the chosen block only
    let block = blocks[selected_x + selected_z * width].0;
    let canvas = chosen_block_canvas_map
        .get(&canvas_name)
        .unwrap()
        .get()
        .unwrap();
    let area = CanvasBackend::with_canvas_object(canvas)
        .unwrap()
        .into_drawing_area();

    area.fill(&RGBColor(113, 113, 114)).unwrap();

    let key_points_x = (0..9).collect::<Vec<i32>>();
    let key_points_z = (0..9).collect::<Vec<i32>>();
    let key_points_y = (0..5).map(|x| x as i32 * 64).collect::<Vec<i32>>();

    let mut chart = ChartBuilder::on(&area)
        .margin(40)
        .build_cartesian_3d(
            (0i32..8i32).with_key_points(key_points_x),
            (0i32..256i32).with_key_points(key_points_y),
            (0i32..8i32).with_key_points(key_points_z),
        )
        .unwrap();

    chart.with_projection(|mut pb| {
        pb.pitch = 1.2;
        pb.yaw = 0.5;
        pb.scale = 0.7;
        pb.into_matrix()
    });

    chart.configure_axes().draw().unwrap();

    chart
        .draw_series(
            (0i32..8i32)
                .map(|x| std::iter::repeat(x).zip(0i32..8i32))
                .flatten()
                .map(|(x, z)| {
                    let value = (block[z as usize][x as usize]).abs().clamp(0, 255) as i32;
                    let face = if value == 0 {
                        TRANSPARENT
                    } else {
                        HSLColor(240.0 / 360.0 - 240.0 / 360.0 * value as f64 / 5.0, 1.0, 0.7)
                            .to_rgba()
                    };

                    let edge = if value == 0 {
                        TRANSPARENT
                    } else {
                        BLACK.to_rgba()
                    };
                    Cubiod::new([(x, 0, z), (x + 1, value, z + 1)], face.filled(), &edge)
                }),
        )
        .unwrap();
}

#[allow(clippy::ptr_arg)]
fn draw_spatial_channel(
    quantized_block: &BlockMatrix,
    canvas_map: &HashMap<CanvasName, ElRef<HtmlCanvasElement>>,
    canvas_name: CanvasName,
    zoom: u32,
) {
    let width = quantized_block.width;
    let height = quantized_block.height;

    let mut image_data = vec![0; width * height * 8 * 8 * 4];

    for v in 0..height {
        for u in 0..width {
            let spatial = &quantized_block.blocks[u + v * width];
            write_to_image_data(&mut image_data, &spatial.0, u, v, width);
        }
    }
    draw_default(&canvas_map, canvas_name, image_data, zoom);
}

#[allow(clippy::too_many_arguments)]
fn draw_ycbcr_recovered(
    canvas_map: &HashMap<CanvasName, ElRef<HtmlCanvasElement>>,
    ys_quantized: &BlockMatrix,
    cbs_quantized: &BlockMatrix,
    crs_quantized: &BlockMatrix,
    subsampling_pack: &SubsamplingPack,
    image_window: &image::RawImageWindow,
    quality: u8,
    zoom: u32,
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

    draw_default(&canvas_map, CanvasName::YsRecovered, ys_image, zoom);
    draw_default(&canvas_map, CanvasName::CbsRecovered, cbs_image, zoom);
    draw_default(&canvas_map, CanvasName::CrsRecovered, crs_image, zoom);

    draw_image_recovered(
        canvas_map,
        ys,
        cbs,
        crs,
        &subsampling_pack,
        image_window,
        zoom,
    );
}

fn draw_image_recovered(
    canvas_map: &HashMap<CanvasName, ElRef<HtmlCanvasElement>>,
    ys: Vec<u8>,
    cbs: Vec<u8>,
    crs: Vec<u8>,
    subsampling_pack: &SubsamplingPack,
    image_window: &image::RawImageWindow,
    zoom: u32,
) {
    let horiz_mult: usize = horiz_mult_from_subsampling(&subsampling_pack);
    let vert_mult: usize = vert_mult_from_subsampling(&subsampling_pack);

    let mut output_image = Vec::<u8>::new();
    for i in 0..ys.len() {
        let curr_cb_cr: usize = subsampled_index_for_recovery(i, horiz_mult, vert_mult);
        let RGB { r, g, b } = image::pixel::YCbCr {
            y: ys[i],
            cb: cbs[curr_cb_cr],
            cr: crs[curr_cb_cr],
        }
        .to_rgb();
        output_image.push(r);
        output_image.push(g);
        output_image.push(b);
        output_image.push(255);
    }

    let input_image = image_window.to_image();
    let image_diff = get_image_diff(&output_image, &input_image);
    draw_default(&canvas_map, CanvasName::ImageRecovered, output_image, zoom);

    draw_default(&canvas_map, CanvasName::Difference, image_diff, zoom);
}
pub fn subsampled_index_for_recovery(i: usize, horiz_mult: usize, vert_mult: usize) -> usize {
    return ((i / (BLOCK_SIZE as usize * vert_mult)) * BLOCK_SIZE as usize
        + i % (BLOCK_SIZE as usize))
        / horiz_mult;
}

fn draw_default(
    canvas_map: &HashMap<CanvasName, ElRef<HtmlCanvasElement>>,
    canvas_name: CanvasName,
    image_data: Vec<u8>,
    zoom: u32,
) {
    let canvas = canvas_map.get(&canvas_name).unwrap();
    draw_scaled_image_default(&canvas, &image_data, zoom);
}

fn write_to_image_data(
    image_data: &mut Vec<u8>,
    spatial: &[[i16; 8]; 8],
    u: usize,
    v: usize,
    vert_block_count: usize,
) {
    for y in 0..8 {
        for x in 0..8 {
            let offset = ((v * 8 + y) * (vert_block_count * 8) as usize + (u * 8) + x) * 4;
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
    turn_antialiasing_off_for_ordinary(canvas_map);
    turn_antialiasing_off_for_preview(preview_canvas_map);
}
fn turn_antialiasing_off_for_ordinary(canvas_map: &HashMap<CanvasName, ElRef<HtmlCanvasElement>>) {
    for (_canvas_name, canvas) in canvas_map {
        turn_antialising_off_for_specific_canvas(&canvas);
    }
}
fn turn_antialiasing_off_for_preview(
    preview_canvas_map: &HashMap<PreviewCanvasName, ElRef<HtmlCanvasElement>>,
) {
    for (_canvas_name, canvas) in preview_canvas_map {
        turn_antialising_off_for_specific_canvas(&canvas);
    }
}
fn turn_antialising_off_for_specific_canvas(canvas: &ElRef<HtmlCanvasElement>) {
    let ctx = canvas_context_2d(&canvas.get().unwrap());
    ctx.set_image_smoothing_enabled(false);
}

fn draw_input_previews(
    preview_canvas_map: &HashMap<PreviewCanvasName, ElRef<HtmlCanvasElement>>,
    image_window: &RawImageWindow,
    zoom: u32,
) {
    let input_image = &image_window.to_image();
    for (_canvas_name, canvas) in preview_canvas_map {
        draw_scaled_image_default(&canvas, &input_image, zoom);
    }
}

fn draw_input_selection_indicator(
    overlay_image_ref: &ElRef<HtmlImageElement>,
    image_window: &RawImageWindow,
    image: &image::RawImage,
    zoom: u32,
) {
    let overlay_image = overlay_image_ref.get().unwrap();

    let tmp_canvas = create_tmp_canvas();

    tmp_canvas.set_width(overlay_image.width());
    tmp_canvas.set_height(overlay_image.height());

    let tmp_ctx = canvas_context_2d(&tmp_canvas);

    // We need to calculate offset of line_width so that all pixels of the image window are inside stroked rect (as in not covered by the lines)
    let line_width: i32 = (4 * zoom / 8) as i32;
    let rect_a =
        (((overlay_image.height() * BLOCK_SIZE) / image.height()) as i32 + line_width) as f64;
    let rect_x = cmp::max(
        0,
        ((image_window.start_x * overlay_image.height()) / image.height()) as i32 - line_width / 2,
    ) as f64;
    let rect_y = cmp::max(
        0,
        ((image_window.start_y * overlay_image.width()) / image.width()) as i32 - line_width / 2,
    ) as f64;

    tmp_ctx.set_line_width(line_width as f64);
    tmp_ctx.stroke_rect(rect_x, rect_y, rect_a, rect_a);

    let f = Closure::once_into_js(move |blob: &Blob| {
        let url = web_sys::Url::create_object_url_with_blob(&blob).unwrap();

        overlay_image.set_src(&url);
        overlay_image.set_onload(Some(
            Closure::once_into_js(move || {
                // no longer need to read the blob so it's revoked
                web_sys::Url::revoke_object_url(&url).unwrap();
            })
            .as_ref()
            .unchecked_ref::<js_sys::Function>(),
        ));
    });
    tmp_canvas
        .to_blob(f.as_ref().unchecked_ref::<js_sys::Function>())
        .unwrap();
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

fn draw_all(model: &mut Model) {
    if let State::ImageView(ref mut pack) = model.state {
        turn_antialiasing_off(&model.canvas_map, &model.preview_canvas_map);

        prepare_original_image_preview(
            &model.original_image_canvas,
            &model.original_image_overlay,
            &pack.raw_image,
        );
        draw_input_selection_indicator(
            &model.original_image_overlay,
            &pack.image_window,
            &pack.raw_image,
            model.zoom,
        );
        draw_original_image_preview(&model.original_image_canvas, &pack.raw_image);

        draw_input_previews(&model.preview_canvas_map, &pack.image_window, model.zoom);
        draw_ycbcr(
            &model.canvas_map,
            &pack.ycbcr,
            &model.subsampling_pack,
            model.zoom,
        );
        draw_dct_quantized(
            &model.canvas_map,
            pack,
            &model.subsampling_pack,
            model.quality,
            model.zoom,
        );
        draw_dct_quantized_plots(
            &pack,
            &model.plot_map,
            &model.chosen_block_plot_map,
            &model.subsampling_pack,
        );
        draw_block_choice_indicators(
            &model.overlay_map,
            &model.preview_overlay_map,
            pack.chosen_block_x,
            pack.chosen_block_y,
            &model.subsampling_pack,
            model.zoom,
        );
    }
}
pub(crate) fn update(msg: Msg, model: &mut Model, orders: &mut impl Orders<Msg>) {
    match msg {
        Msg::FileChooserLoadImage(file) => {
            let file_blob = gloo_file::Blob::from(file);
            orders.perform_cmd(async move {
                let raw_image = utils::load_image(file_blob).await;
                Msg::ImageLoaded(raw_image)
            });
            model.quality = 50;
            model.zoom = 7;
            model.state = State::PreImageView
        }
        Msg::FileChooserDragStarted => model.file_chooser_zone_active = true,
        Msg::FileChooserDragLeave => model.file_chooser_zone_active = false,
        Msg::ImageLoaded(raw_image) => {
            let raw_image_rc = Rc::new(raw_image);
            let image_window =
                RawImageWindow::new(raw_image_rc.clone(), 0, 0, BLOCK_SIZE, BLOCK_SIZE);

            let ycbcr = image_window.to_rgb_image().to_ycbcr_image();
            let pack: ImagePack = ImagePack {
                raw_image: raw_image_rc,
                image_window,
                ycbcr,
                plot_data: HashMap::<PlotName, BlockMatrix>::new(),
                chosen_block_x: 0.0,
                chosen_block_y: 0.0,
            };
            model.state = State::ImageView(pack);

            draw_all(model);
        }
        Msg::DiffInfoDisplayChanged => {
            model.is_diff_info_shown = !model.is_diff_info_shown;
        }
        Msg::ZoomUpdated(zoom) => {
            model.zoom = zoom;
            orders.after_next_render(|_| Msg::PostZoomUpdated);
        }
        Msg::PostZoomUpdated => {
            draw_all(model);
        }
        Msg::QualityUpdated(quality) => {
            if let State::ImageView(ref mut pack) = model.state {
                model.quality = quality;
                draw_dct_quantized(
                    &model.canvas_map,
                    pack,
                    &model.subsampling_pack,
                    quality,
                    model.zoom,
                );
                draw_dct_quantized_plots(
                    &pack,
                    &model.plot_map,
                    &model.chosen_block_plot_map,
                    &model.subsampling_pack,
                );
            }
        }
        Msg::PreviewCanvasClicked(x, y) => {
            if let State::ImageView(ref mut pack) = model.state {
                let preview_canvas_ref = &model.original_image_canvas;
                let preview_canvas = preview_canvas_ref.get().unwrap();
                let canvas_rect = preview_canvas.get_bounding_client_rect();

                let canvas_x = canvas_rect.left();
                let canvas_y = canvas_rect.top();

                let image_click_x: i32 = ((x - canvas_x as i32) as u32 * pack.raw_image.height()
                    / (BLOCK_SIZE * model.zoom)) as i32;
                let image_click_y: i32 = ((y - canvas_y as i32) as u32 * pack.raw_image.height()
                    / (BLOCK_SIZE * model.zoom)) as i32;

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

                pack.ycbcr = pack.image_window.to_rgb_image().to_ycbcr_image();

                draw_input_selection_indicator(
                    &model.original_image_overlay,
                    &pack.image_window,
                    &pack.raw_image,
                    model.zoom,
                );

                draw_input_previews(&model.preview_canvas_map, &pack.image_window, model.zoom);
                draw_ycbcr(
                    &model.canvas_map,
                    &pack.ycbcr,
                    &model.subsampling_pack,
                    model.zoom,
                );
                draw_dct_quantized(
                    &model.canvas_map,
                    pack,
                    &model.subsampling_pack,
                    model.quality,
                    model.zoom,
                );
                draw_dct_quantized_plots(
                    &pack,
                    &model.plot_map,
                    &model.chosen_block_plot_map,
                    &model.subsampling_pack,
                );
            }
        }
        Msg::BlockChosen(x, y, rect_x, rect_y, is_resizable_canvas) => {
            if let State::ImageView(ref mut pack) = model.state {
                let vert_mult: usize = if !is_resizable_canvas {
                    1_usize
                } else {
                    vert_mult_from_subsampling(&model.subsampling_pack)
                };
                let horiz_mult: usize = if !is_resizable_canvas {
                    1_usize
                } else {
                    horiz_mult_from_subsampling(&model.subsampling_pack)
                };

                let start_x: f64 = cmp::min(
                    (x - rect_x) - (x - rect_x) % (8 * model.zoom as i32),
                    (((BLOCK_SIZE as usize - 8 * horiz_mult) * model.zoom as usize) / horiz_mult)
                        as i32,
                ) as f64
                    * horiz_mult as f64;
                let start_y: f64 = cmp::min(
                    (y - rect_y) - (y - rect_y) % (8 * model.zoom as i32),
                    (((BLOCK_SIZE as usize - 8 * vert_mult) * model.zoom as usize) / vert_mult)
                        as i32,
                ) as f64
                    * vert_mult as f64;

                // chosen_block_x_y are coords if zoom was equal 1
                pack.chosen_block_x = start_x / model.zoom as f64;
                pack.chosen_block_y = start_y / model.zoom as f64;

                draw_block_choice_indicators(
                    &model.overlay_map,
                    &model.preview_overlay_map,
                    pack.chosen_block_x,
                    pack.chosen_block_y,
                    &model.subsampling_pack,
                    model.zoom,
                );

                draw_dct_quantized_plots(
                    &pack,
                    &model.plot_map,
                    &model.chosen_block_plot_map,
                    &model.subsampling_pack,
                );
            }
        }
        Msg::SubsamplingRatioChanged(y_ratio, cb_ratio, cr_ratio) => {
            if let State::ImageView(_) = model.state {
                model.subsampling_pack.j = y_ratio;
                model.subsampling_pack.a = cb_ratio;
                model.subsampling_pack.b = cr_ratio;

                orders.after_next_render(|_| Msg::PostSubsamplingRatioChanged);
            }
        }
        Msg::PostSubsamplingRatioChanged => {
            if let State::ImageView(ref mut pack) = model.state {
                turn_antialiasing_off_for_ordinary(&model.canvas_map);

                draw_ycbcr(
                    &model.canvas_map,
                    &pack.ycbcr,
                    &model.subsampling_pack,
                    model.zoom,
                );
                draw_dct_quantized(
                    &model.canvas_map,
                    pack,
                    &model.subsampling_pack,
                    model.quality,
                    model.zoom,
                );
                draw_block_choice_indicators(
                    &model.overlay_map,
                    &model.preview_overlay_map,
                    pack.chosen_block_x,
                    pack.chosen_block_y,
                    &model.subsampling_pack,
                    model.zoom,
                );
                draw_dct_quantized_plots(
                    &pack,
                    &model.plot_map,
                    &model.chosen_block_plot_map,
                    &model.subsampling_pack,
                );
            }
        }
    }
}
