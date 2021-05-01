use seed::*;
use seed::prelude::*;

use crate::{Msg as GMsg, image, quant, block};

use seed::prelude::web_sys::{DragEvent, Event};
use web_sys::HtmlCanvasElement;

trait IntoDragEvent {
    fn into_drag_event(self) -> DragEvent;
}

impl IntoDragEvent for Event {
    fn into_drag_event(self) -> DragEvent {
        self.dyn_into::<web_sys::DragEvent>()
            .expect("cannot cast given event into DragEvent")
    }
}

macro_rules! stop_and_prevent {
    { $event:expr } => {
        {
            $event.stop_propagation();
            $event.prevent_default();
        }
     };
}


// ------ ------
//   Messages
// ------ ------

struct ImagePack{
    raw_image: image::RawImage,
    ycbcr: image::YCbCrImage,
}

enum State {
    FileChooser,
    PreImageView,
    ImageView(ImagePack)
}

// ------ ------
//   Messages
// ------ ------

#[derive(Clone)]
pub enum Msg {
    FileChooserLoadImage(web_sys::File),
    FileChooserDragStarted,
    FileChooserDragLeave,
    ImageLoaded(image::RawImage),
    QualityUpdated(u8),
}

// ------ ------
//     Model
// ------ ------

pub struct Model {
    file_chooser_zone_active: bool,
    base_url: Url,
    state: State,
    original_canvas: ElRef<HtmlCanvasElement>,
    ys_canvas: ElRef<HtmlCanvasElement>,
    cbs_canvas: ElRef<HtmlCanvasElement>,
    crs_canvas: ElRef<HtmlCanvasElement>,
    ys_quant_canvas: ElRef<HtmlCanvasElement>,
    cbs_quant_canvas: ElRef<HtmlCanvasElement>,
    crs_quant_canvas: ElRef<HtmlCanvasElement>,

    quality: u8,
}

pub fn init(mut url: Url) -> Option<Model> {
    let base_url = url.to_base_url();

    Some(Model {
        file_chooser_zone_active: false,
        base_url,
        state: State::FileChooser,
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

fn draw_original_image(canvas: &ElRef<HtmlCanvasElement>, image: &image::RawImage) {
    let canvas = canvas.get().unwrap();
    let ctx = canvas_context_2d(&canvas);
    let img = web_sys::ImageData::new_with_u8_clamped_array(wasm_bindgen::Clamped(&image.0), 500).unwrap();
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


fn view_image_preview(model: &Model) -> Node<GMsg> {
    div![
        C!["image_view"],
        details![
            summary!["Image preview"],
            canvas![
                el_ref(&model.original_canvas),
                attrs![
                    At::Width => px(500),
                    At::Height => px(500),
                ]
            ]
        ]
    ]
}

fn view_ycbcr(model: &Model) -> Node<GMsg> {
    div![
        C!["image_view"],
        details![
            summary!["YCbCr"],
            canvas![
                el_ref(&model.ys_canvas),
                attrs![
                    At::Width => px(500),
                    At::Height => px(500),
                ]
            ],
            canvas![
                el_ref(&model.cbs_canvas),
                attrs![
                    At::Width => px(500),
                    At::Height => px(500),
                ]
            ],
            canvas![
                el_ref(&model.crs_canvas),
                attrs![
                    At::Width => px(500),
                    At::Height => px(500),
                ]
            ]
        ]
    ]
}

fn view_dct_quantized(model: &Model) -> Node<GMsg> {
    div![
        C!["image_view"],
        details![
            summary!["DCT Quantized"],
            canvas![
                el_ref(&model.ys_quant_canvas),
                attrs![
                    At::Width => px(500),
                    At::Height => px(500),
                ]
            ],
            canvas![
                el_ref(&model.cbs_quant_canvas),
                attrs![
                    At::Width => px(500),
                    At::Height => px(500),
                ]
            ],
            canvas![
                el_ref(&model.crs_quant_canvas),
                attrs![
                    At::Width => px(500),
                    At::Height => px(500),
                ]
            ]
        ]
    ]
}

fn view_settings_sidebar(model: &Model) -> Node<GMsg> {
    div![
        C!["setting_sidebar"],
        input![
            C!["sidebar_activator"],
            attrs!{
                At::Type => "checkbox",
                At::Id => "sidebar_activator",
                At::Name => "sidebar_activator",
            }
        ],
        label![
            C!["sidebar_activator"],
            attrs!{
                At::For => "sidebar_activator"
            },
            span![]
        ],
        div![
            C!["sidebar_settings"],
            label![
                attrs!{
                    At::For => "quality"
                },
                "Quality:"
            ],
            input![
                attrs!{
                    At::Type => "range",
                    At::Max => 100,
                    At::Min => 1,
                    At::Id => "quality",
                },
                input_ev("change", |value| {
                    wrap(Msg::QualityUpdated(value.parse::<u8>().unwrap()))
                })
            ]
        ]
    ]
}

fn view_jpeg_visualization(model: &Model) -> Node<GMsg> {
    div![
        view_settings_sidebar(&model),
        view_image_preview(&model),
        view_ycbcr(&model),
        view_dct_quantized(&model)
    ]
}

fn view_file_chooser(model: &Model) -> Node<GMsg> {
    div![
        C!["choose_file_wrapper"],
        div![
            C![
                "drop_area_wrapper",
                IF!(model.file_chooser_zone_active => "drop_active"),
            ],
            div![
                C!["drop_area"],
                input![
                    C!["drop_file"],
                    attrs!{
                        At::Type => "file",
                        At::Id => "file",
                        At::Name => "file",
                    },
                    ev(Ev::Change, |event| {
                        let file = event
                            .target()
                            .and_then(|target| target.dyn_into::<web_sys::HtmlInputElement>().ok())
                            .and_then(|file_input| file_input.files())
                            .and_then(|file_list| file_list.get(0)).unwrap();
                        wrap(Msg::FileChooserLoadImage(file))
                    }),
                ],
                label![
                    attrs!{
                        At::For => "file"
                    },
                    strong![
                        "Choose a file"
                    ],
                    " or drag it here"
                ],
            ],
            ev(Ev::DragEnter, |event| {
                stop_and_prevent!(event);
                wrap(Msg::FileChooserDragStarted)
            }),
            ev(Ev::DragOver, |event| {
                let drag_event = event.into_drag_event();
                stop_and_prevent!(drag_event);
                drag_event.data_transfer().unwrap().set_drop_effect("copy");
            }),
            ev(Ev::DragLeave, |event| {
                stop_and_prevent!(event);
                wrap(Msg::FileChooserDragLeave)
            }),
            ev(Ev::Drop, |event| {
                let drag_event = event.into_drag_event();
                stop_and_prevent!(drag_event);
                let file = drag_event.data_transfer()
                    .and_then(|file_input| file_input.files())
                    .and_then(|files| files.get(0)).unwrap();
                wrap(Msg::FileChooserLoadImage(file))
            })

        ],

    ]
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
                let canvas = web_sys::window().unwrap()
                    .document().unwrap()
                    .create_element("canvas").unwrap()
                    .dyn_into::<web_sys::HtmlCanvasElement>().unwrap();
                canvas.set_width(500);
                canvas.set_height(500);
                let context = canvas.get_context("2d")
                    .unwrap().unwrap()
                    .dyn_into::<web_sys::CanvasRenderingContext2d>()
                    .unwrap();
                context.draw_image_with_html_image_element(&image, 0.0, 0.0).unwrap();
                let image_data = context.get_image_data(0.0, 0.0, 500.0, 500.0).unwrap();
                let data: Vec<u8> = image_data.data().to_vec();
                Msg::ImageLoaded(image::RawImage(data))
            });
            model.state = State::PreImageView
        },
        Msg::FileChooserDragStarted => model.file_chooser_zone_active = true,
        Msg::FileChooserDragLeave => model.file_chooser_zone_active = false,
        Msg::ImageLoaded(raw_image) => {
            let ycbcr =  raw_image.to_rgb_image().to_ycbcr_image();
            draw_original_image(&model.original_canvas, &raw_image);
            draw_ycbcr(&model.ys_canvas, &model.cbs_canvas, &model.crs_canvas, &ycbcr);
            draw_dct_quantized(&model.ys_quant_canvas, &model.cbs_quant_canvas, &model.crs_quant_canvas, &ycbcr, 50);
            model.state = State::ImageView(ImagePack{
                raw_image,
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