use seed::*;
use seed::prelude::*;

use crate::{Msg as GMsg, image, quant, block};
use web_sys::HtmlCanvasElement;
use super::model::*;
use super::view::*;


pub fn init(mut url: Url) -> Option<Model> {
    let base_url = url.to_base_url();

    Some(Model {
        file_chooser_zone_active: false,
        base_url,
        state: State::FileChooser,
        original_canvas_preview: ElRef::<HtmlCanvasElement>::default(),
        original_canvas: ElRef::<HtmlCanvasElement>::default(),
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
        State::ImageView(raw_image) => view_jpeg_visualization(model),
    }
}

pub fn wrap(msg: Msg) -> GMsg {
    GMsg::JPEGVisualizationMessage(msg)
}

fn draw_original_image_preview(original_canvas_preview: &ElRef<HtmlCanvasElement>, image: &image::RawImage) {
    let canvas = original_canvas_preview.get().unwrap();
    let ctx = canvas_context_2d(&canvas);
    let img = web_sys::ImageData::new_with_u8_clamped_array(wasm_bindgen::Clamped(&image.all_data), image.width).unwrap();

    // Create temporary canvas1 so that we can draw scaled image to proper canvas
    let tmp_canvas = web_sys::window().unwrap()
        .document().unwrap()
        .create_element("canvas").unwrap()
        .dyn_into::<web_sys::HtmlCanvasElement>().unwrap();
    tmp_canvas.set_height(image.height);
    tmp_canvas.set_width(image.width);
    let tmp_ctx = canvas_context_2d(&tmp_canvas);
    tmp_ctx.put_image_data(&img, 0.0, 0.0).unwrap();

    // Set scale and draw scaled image from temporary canvas1
    ctx.scale(500.0/image.width as f64, 500.0/image.height as f64);
    ctx.draw_image_with_html_canvas_element(&tmp_canvas, 0.0, 0.0);
}
fn draw_original_image(canvas: &ElRef<HtmlCanvasElement>, image: &image::RawImage) {
    let canvas = canvas.get().unwrap();
    canvas.set_height(image.height);
    canvas.set_width(image.width);
    let ctx = canvas_context_2d(&canvas);
    let img = web_sys::ImageData::new_with_u8_clamped_array_and_sh(wasm_bindgen::Clamped(&image.all_data), image.width, image.height).unwrap();
    ctx.put_image_data(&img, 0.0, 0.0).unwrap();
}

fn draw_ycbcr(canvas_ys: &ElRef<HtmlCanvasElement>, canvas_cbs: &ElRef<HtmlCanvasElement>, canvas_crs: &ElRef<HtmlCanvasElement>, image: &image::YCbCrImage) {
    let ctx_ys = canvas_context_2d(&canvas_ys.get().unwrap());
    let ctx_cbs = canvas_context_2d(&canvas_cbs.get().unwrap());
    let ctx_crs = canvas_context_2d(&canvas_crs.get().unwrap());

    let ys = image.to_ys_channel();
    let cbs  = image.to_cbs_channel();
    let crs = image.to_crs_channel();

    let ys_image = ys
        .iter()
        .flat_map(|x| {
            let (r, g, b) = image::pixel::YCbCr((*x, 128, 128)).to_rgb().0;
            vec![r, g, b, 255]
        })
        .collect::<Vec<u8>>();

    let cbs_image = cbs
        .iter()
        .flat_map(|x| {
            let (r, g, b) = image::pixel::YCbCr((128, *x, 128)).to_rgb().0;
            vec![r, g, b, 255]
        })
        .collect::<Vec<u8>>();

    let crs_image = crs
        .iter()
        .flat_map(|x| {
            let (r, g, b) = image::pixel::YCbCr((128, 128, *x)).to_rgb().0;
            vec![r, g, b, 255]
        })
        .collect::<Vec<u8>>();

    let ys =
        web_sys::ImageData::new_with_u8_clamped_array(wasm_bindgen::Clamped(&ys_image), 500).unwrap();
    ctx_ys.put_image_data(&ys, 0.0, 0.0).unwrap();
    let cbs =
        web_sys::ImageData::new_with_u8_clamped_array(wasm_bindgen::Clamped(&cbs_image), 500).unwrap();
    ctx_cbs.put_image_data(&cbs, 0.0, 0.0).unwrap();
    let crs =
        web_sys::ImageData::new_with_u8_clamped_array(wasm_bindgen::Clamped(&crs_image), 500).unwrap();
    ctx_crs.put_image_data(&crs, 0.0, 0.0).unwrap();
}

fn draw_dct_quantized(canvas_ys: &ElRef<HtmlCanvasElement>, canvas_cbs: &ElRef<HtmlCanvasElement>, canvas_crs: &ElRef<HtmlCanvasElement>, image: &image::YCbCrImage, quality: u8) {
    let ctx_ys = canvas_context_2d(&canvas_ys.get().unwrap());
    let ctx_cbs = canvas_context_2d(&canvas_cbs.get().unwrap());
    let ctx_crs = canvas_context_2d(&canvas_crs.get().unwrap());

    let ys = image.to_ys_channel();
    let cbs  = image.to_cbs_channel();
    let crs = image.to_crs_channel();

    let scaled_luminance_quant_table = quant::scale_quantization_table(&quant::LUMINANCE_QUANTIZATION_TABLE, quality);
    let scaled_chrominance_quant_table = quant::scale_quantization_table(&quant::CHROMINANCE_QUANTIZATION_TABLE, quality);

    let ys_block_matrix = block::split_to_block_matrix(&ys);
    let cbs_block_matrix = block::split_to_block_matrix(&cbs);
    let crs_block_matrix = block::split_to_block_matrix(&crs);

    let ys_quantized = ys_block_matrix.apply_quantization(&scaled_luminance_quant_table);
    let cbs_quantized = cbs_block_matrix.apply_quantization(&scaled_chrominance_quant_table);
    let crs_quantized = crs_block_matrix.apply_quantization(&scaled_chrominance_quant_table);

    draw_spatial_channel(&ys_quantized, ys_block_matrix.width, ys_block_matrix.height, &ctx_ys);
    draw_spatial_channel(&cbs_quantized, cbs_block_matrix.width, cbs_block_matrix.height, &ctx_cbs);
    draw_spatial_channel(&crs_quantized, crs_block_matrix.width, crs_block_matrix.height, &ctx_crs);
}

fn draw_spatial_channel(
    data: &Vec<[[u8; 8]; 8]>,
    width: usize,
    height: usize,
    canvas_context: &web_sys::CanvasRenderingContext2d,
) {
    let mut image_data = vec![0; 500 * 500 * 4];

    for v in 0..width {
        for u in 0..height {
            let mut spatial = data[u + v * width];
            write_to_image_data(&mut image_data, &spatial, u, v);
        }
    }

    let image_data =
        web_sys::ImageData::new_with_u8_clamped_array(wasm_bindgen::Clamped(&image_data), 500)
            .unwrap();

    canvas_context
        .put_image_data(&image_data, 0.0, 0.0)
        .unwrap();
}

fn write_to_image_data(image_data: &mut Vec<u8>, spatial: &[[u8; 8]; 8], u: usize, v: usize) {
    for y in 0..8 {
        for x in 0..8 {
            let offset = ((v * 8 + y) * 500 + (u * 8) + x) * 4;
            image_data[offset] = 255 - spatial[y][x];
            image_data[offset + 1] = 255 - spatial[y][x];
            image_data[offset + 2] = 255 - spatial[y][x];
            image_data[offset + 3] = 255;
        }
    }
}



struct_urls!();
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
                let url_data = gloo_file::futures::read_as_data_url(&file_blob).await.unwrap();
                let image = web_sys::HtmlImageElement::new().unwrap();
                image.set_src(&url_data);
                JsFuture::from(image.decode()).await;
                let image_height = image.natural_height();
                let image_width = image.natural_width();
                let canvas = web_sys::window().unwrap()
                    .document().unwrap()
                    .create_element("canvas").unwrap()
                    .dyn_into::<web_sys::HtmlCanvasElement>().unwrap();
                canvas.set_width(image_width);
                canvas.set_height(image_height);
                let context = canvas.get_context("2d")
                    .unwrap().unwrap()
                    .dyn_into::<web_sys::CanvasRenderingContext2d>()
                    .unwrap();
                context.draw_image_with_html_image_element(&image, 0.0, 0.0).unwrap();
                let image_data = context.get_image_data(0.0, 0.0, image_width.into(), image_height.into()).unwrap();
                let data: Vec<u8> = image_data.data().to_vec();
                let mut raw_image: image::RawImage = image::RawImage::new(data, image_height, image_width);
                //raw_image.move_viewed(250, 250);
                Msg::ImageLoaded(raw_image)
            });
            model.state = State::PreImageView
        },
        Msg::FileChooserDragStarted => model.file_chooser_zone_active = true,
        Msg::FileChooserDragLeave => model.file_chooser_zone_active = false,
        Msg::ImageLoaded(raw_image) => {
            let ycbcr =  raw_image.to_rgb_image().to_ycbcr_image();
            draw_original_image_preview(&model.original_canvas_preview, &raw_image);
            draw_original_image(&model.original_canvas, &raw_image);
            draw_ycbcr(&model.ys_canvas, &model.cbs_canvas, &model.crs_canvas, &ycbcr);
            draw_dct_quantized(&model.ys_quant_canvas, &model.cbs_quant_canvas, &model.crs_quant_canvas, &ycbcr, 50);
            model.state = State::ImageView(ImagePack{
                raw_image,
                start_x: 0,
                start_y: 0,
                ycbcr
            });
        },
        Msg::QualityUpdated(quality) => {
            if let State::ImageView(pack) = &model.state {
                model.quality = quality;
                draw_dct_quantized(&model.ys_quant_canvas, &model.cbs_quant_canvas, &model.crs_quant_canvas, &pack.ycbcr, quality);
            }
        }
    }
}