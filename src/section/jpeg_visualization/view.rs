use seed::prelude::web_sys::MouseEvent;
use seed::prelude::*;
use seed::*;

use super::model::{CanvasName, Model, Msg, PreviewCanvasName, State};
use super::page::wrap;
use crate::graphic_helpers::drag_n_drop::*;
use crate::section::jpeg_visualization::model::{PlotName, SubsamplingPack, ExampleImage};
use crate::{Msg as GMsg, BLOCK_SIZE};
use web_sys::{Event, HtmlCanvasElement, HtmlImageElement};

macro_rules! stop_and_prevent {
    { $event:expr } => {
        {
            $event.stop_propagation();
            $event.prevent_default();
        }
     };
}

pub fn view_image_preview(model: &Model) -> Node<GMsg> {
    div![
        C!["image_view"],
        details![
            summary!["Image preview"],
            attrs![
                At::Open => match &model.state {
                    State::ImageView(_pack) => AtValue::None,
                    _ => AtValue::Ignored,
                }
            ],
            div![
                C!["canvas_with_overlay_container"],
                img![
                    C!["canvas_overlay"],
                    el_ref(&model.original_image_overlay),
                    attrs![
                        At::Width => px(BLOCK_SIZE * model.zoom),
                        At::Height => px(BLOCK_SIZE * model.zoom),
                        At::Draggable => false,
                    ],
                    ev(Ev::Click, |event| {
                        let mouse_event: MouseEvent = event.unchecked_into();
                        wrap(Msg::PreviewCanvasClicked(mouse_event.x(), mouse_event.y()))
                    }),
                ],
                canvas![
                    el_ref(&model.original_image_canvas),
                    attrs![
                        At::Width => px(BLOCK_SIZE * model.zoom),
                        At::Height => px(BLOCK_SIZE * model.zoom),
                    ],
                ],
            ],
            canvas_labeled_div_with_overlay(
                "INPUT",
                &model
                    .preview_canvas_map
                    .get(&PreviewCanvasName::Original)
                    .unwrap(),
                &model
                    .preview_overlay_map
                    .get(&PreviewCanvasName::Original)
                    .unwrap(),
                None,
                model.zoom,
            ),
        ]
    ]
}

pub fn view_ycbcr(model: &Model) -> Node<GMsg> {
    div![
        C!["image_view"],
        details![
            summary!["YCbCr"],
            canvas_labeled_div_with_overlay(
                "INPUT",
                &model
                    .preview_canvas_map
                    .get(&PreviewCanvasName::YCbCr)
                    .unwrap(),
                &model
                    .preview_overlay_map
                    .get(&PreviewCanvasName::YCbCr)
                    .unwrap(),
                None,
                model.zoom
            ),
            canvas_labeled_div_with_overlay(
                " Y ",
                &model.canvas_map.get(&CanvasName::Ys).unwrap(),
                &model.overlay_map.get(&CanvasName::Ys).unwrap(),
                None,
                model.zoom
            ),
            canvas_labeled_div_with_overlay(
                "CB",
                &model.canvas_map.get(&CanvasName::Cbs).unwrap(),
                &model.overlay_map.get(&CanvasName::Cbs).unwrap(),
                Some(&model.subsampling_pack),
                model.zoom
            ),
            canvas_labeled_div_with_overlay(
                "CR",
                &model.canvas_map.get(&CanvasName::Crs).unwrap(),
                &model.overlay_map.get(&CanvasName::Crs).unwrap(),
                Some(&model.subsampling_pack),
                model.zoom
            ),
        ]
    ]
}

pub fn view_dct_quantized(model: &Model) -> Node<GMsg> {
    div![
        C!["image_view"],
        details![
            summary!["DCT Quantized"],
            div![
                C!["row_of_columns"],
                canvas_labeled_div_with_overlay(
                    "INPUT",
                    &model
                        .preview_canvas_map
                        .get(&PreviewCanvasName::YCbCrQuant)
                        .unwrap(),
                    &model
                        .preview_overlay_map
                        .get(&PreviewCanvasName::YCbCrQuant)
                        .unwrap(),
                    None,
                    model.zoom
                ),
                div![
                    C!["canvas_column_wrapper"],
                    canvas_labeled_div_with_overlay(
                        "Y QUANTIZED",
                        &model.canvas_map.get(&CanvasName::YsQuant).unwrap(),
                        &model.overlay_map.get(&CanvasName::YsQuant).unwrap(),
                        None,
                        model.zoom
                    ),
                    plot_labeled_div(
                        "Y QUANTIZED 3D",
                        &model.plot_map.get(&PlotName::YsQuant3d).unwrap(),
                        model.zoom
                    ),
                    plot_labeled_div(
                        "Y QUANTIZED 3D CHOSEN BLOCK",
                        &model
                            .chosen_block_plot_map
                            .get(&PlotName::YsQuant3d)
                            .unwrap(),
                        model.zoom
                    ),
                    style![
                        St::MaxWidth => px(BLOCK_SIZE * model.zoom + 20)
                    ]
                ],
                div![
                    C!["canvas_column_wrapper"],
                    canvas_labeled_div_with_overlay(
                        "CB QUANTIZED",
                        &model.canvas_map.get(&CanvasName::CbsQuant).unwrap(),
                        &model.overlay_map.get(&CanvasName::CbsQuant).unwrap(),
                        Some(&model.subsampling_pack),
                        model.zoom
                    ),
                    plot_labeled_div(
                        "CB QUANTIZED 3D",
                        &model.plot_map.get(&PlotName::CbsQuant3d).unwrap(),
                        model.zoom
                    ),
                    plot_labeled_div(
                        "CB QUANTIZED 3D CHOSEN BLOCK",
                        &model
                            .chosen_block_plot_map
                            .get(&PlotName::CbsQuant3d)
                            .unwrap(),
                        model.zoom
                    ),
                    style![
                        St::MaxWidth => px(BLOCK_SIZE * model.zoom + 20)
                    ]
                ],
                div![
                    C!["canvas_column_wrapper"],
                    canvas_labeled_div_with_overlay(
                        "CR QUANTIZED",
                        &model.canvas_map.get(&CanvasName::CrsQuant).unwrap(),
                        &model.overlay_map.get(&CanvasName::CrsQuant).unwrap(),
                        Some(&model.subsampling_pack),
                        model.zoom
                    ),
                    plot_labeled_div(
                        "CR QUANTIZED 3D",
                        &model.plot_map.get(&PlotName::CrsQuant3d).unwrap(),
                        model.zoom
                    ),
                    plot_labeled_div(
                        "CR QUANTIZED 3D CHOSEN BLOCK",
                        &model
                            .chosen_block_plot_map
                            .get(&PlotName::CrsQuant3d)
                            .unwrap(),
                        model.zoom
                    ),
                    style![
                        St::MaxWidth => px(BLOCK_SIZE * model.zoom + 20)
                    ]
                ],
            ],
        ]
    ]
}

fn view_ycbcr_recovered(model: &Model) -> Node<GMsg> {
    div![
        C!["image_view"],
        details![
            summary!["YCbCr recovered from quantized DCT"],
            canvas_labeled_div_with_overlay(
                "INPUT",
                &model
                    .preview_canvas_map
                    .get(&PreviewCanvasName::YCbCrRecovered)
                    .unwrap(),
                &model
                    .preview_overlay_map
                    .get(&PreviewCanvasName::YCbCrRecovered)
                    .unwrap(),
                None,
                model.zoom
            ),
            canvas_labeled_div_with_overlay(
                "Y RECOVERED",
                &model.canvas_map.get(&CanvasName::YsRecovered).unwrap(),
                &model.overlay_map.get(&CanvasName::YsRecovered).unwrap(),
                None,
                model.zoom
            ),
            canvas_labeled_div_with_overlay(
                "CB RECOVERED",
                &model.canvas_map.get(&CanvasName::CbsRecovered).unwrap(),
                &model.overlay_map.get(&CanvasName::CbsRecovered).unwrap(),
                Some(&model.subsampling_pack),
                model.zoom
            ),
            canvas_labeled_div_with_overlay(
                "CR RECOVERED",
                &model.canvas_map.get(&CanvasName::CrsRecovered).unwrap(),
                &model.overlay_map.get(&CanvasName::CrsRecovered).unwrap(),
                Some(&model.subsampling_pack),
                model.zoom
            ),
        ]
    ]
}

fn view_image_recovered(model: &Model) -> Node<GMsg> {
    div![
        C!["image_view image_recov_and_diff"],
        details![
            summary!["Recovered image and comparison"],
            canvas_labeled_div_with_overlay(
                "INPUT",
                &model
                    .preview_canvas_map
                    .get(&PreviewCanvasName::ForComparison)
                    .unwrap(),
                &model
                    .preview_overlay_map
                    .get(&PreviewCanvasName::ForComparison)
                    .unwrap(),
                None,
                model.zoom
            ),
            canvas_labeled_div_with_overlay(
                "OUTPUT",
                &model.canvas_map.get(&CanvasName::ImageRecovered).unwrap(),
                &model.overlay_map.get(&CanvasName::ImageRecovered).unwrap(),
                None,
                model.zoom
            ),
            labeled_canvas_wrapper(
                BLOCK_SIZE * model.zoom,
                div![
                    C!["label_span_wrapper"],
                    label![
                        C!["canvas_label"],
                        "DIFFERENCE"
                    ],
                    span![
                        C!["question_mark_span"],
                        "?",
                        ev(Ev::Click, move |_| wrap(Msg::DiffInfoDisplayChanged)),
                    ],
                    div![
                        C!["info_overlay"],
                        "Difference for each pixel is percentage equal to sum of differences of (r,g,b) pixel values divided by max possible value (3 * 255)",
                        style![
                            St::Width => px(BLOCK_SIZE * model.zoom * 2 / 3),
                            St::Left => px(BLOCK_SIZE * model.zoom / 6),
                            St::FontSize => em(f64::max(1.0, model.zoom as f64 * 0.24)),
                            St::Display => if model.is_diff_info_shown { "block" } else { "none" },
                        ]
                    ],
                    style![
                        St::Width => px(BLOCK_SIZE * model.zoom)
                    ]
                ],
                canvas_with_overlay(
                    &model.canvas_map.get(&CanvasName::Difference).unwrap(),
                    &model.overlay_map.get(&CanvasName::Difference).unwrap(),
                    None,
                    model.zoom
                ),
            ),
            div![
                C!["labeled_canvas_wrapper diff_scale_with_label_wrapper"],
                label![
                    C!["canvas_label"],
                    "DIFF SCALE"
                ],
                div![
                    C!["diff_scale_wrapper"],
                    div![
                        C!["diff_scale"],
                        style![
                            St::Width => px(32),
                            St::Height => px(BLOCK_SIZE * model.zoom),
                        ]
                    ],
                    div![
                        C!["diff_scale_labels"],
                        label![
                            C!["diff_scale_upper_label"],
                            "33%"
                        ],
                        label![
                            C!["diff_scale_lower_label"],
                            "0%"
                        ],
                        style![
                            St::Height => px(BLOCK_SIZE * model.zoom),
                        ]
                    ]
                ],
            ]
        ]
    ]
}

fn canvas_labeled_div_with_overlay(
    label: &str,
    canvas: &ElRef<HtmlCanvasElement>,
    img: &ElRef<HtmlImageElement>,
    // if canvas should not be subsampled, pass None
    subsampling_pack_option: Option<&SubsamplingPack>,
    zoom: u32,
) -> Node<GMsg> {
    let mut width: u32 = BLOCK_SIZE * zoom;
    let mut height: u32 = BLOCK_SIZE * zoom;

    let mut is_resizable_canvas: bool = false;

    match subsampling_pack_option {
        Some(subsampling_pack) => {
            is_resizable_canvas = true;
            width = width * subsampling_pack.a as u32 / subsampling_pack.j as u32;
            if subsampling_pack.b == 0 {
                height = height / 2;
            }
        }
        None => {}
    }

    labeled_canvas_wrapper(
        BLOCK_SIZE * zoom,
        div![
            label![C!["canvas_label"], &label],
            style![
                St::Width => px(BLOCK_SIZE * zoom),
            ]
        ],
        canvas_with_overlay_with_w_h(canvas, img, width, height, is_resizable_canvas),
    )
}
fn labeled_canvas_wrapper(
    width: u32,
    label_element: Node<GMsg>,
    content_element: Node<GMsg>,
) -> Node<GMsg> {
    let padding = 10;

    div![
        C!["labeled_canvas_wrapper"],
        label_element,
        content_element,
        style![
            St::MaxWidth => px(width + padding * 2),
            // TODO: Instead of that put dct divs into a table
            St::MinHeight => px(width),
        ]
    ]
}

fn canvas_with_overlay_with_w_h(
    canvas: &ElRef<HtmlCanvasElement>,
    img: &ElRef<HtmlImageElement>,
    width: u32,
    height: u32,
    is_resizable_canvas: bool,
) -> Node<GMsg> {
    let cloned_canvas_ref = canvas.clone();

    div![
        C!["canvas_with_overlay_container"],
        img![
            C!["canvas_overlay"],
            el_ref(&img),
            attrs![
                At::Width => px(width),
                At::Height => px(height),
                At::Draggable => false,
            ],
            ev(Ev::Click, move |event: Event| {
                let mouse_event: MouseEvent = event.unchecked_into();
                let cloned_canvas = cloned_canvas_ref.get().unwrap();
                let canvas_rect = cloned_canvas.get_bounding_client_rect();
                wrap(Msg::BlockChosen(
                    mouse_event.x(),
                    mouse_event.y(),
                    canvas_rect.left() as i32,
                    canvas_rect.top() as i32,
                    is_resizable_canvas,
                ))
            }),
        ],
        canvas![
            el_ref(&canvas),
            attrs![
                At::Width => px(width),
                At::Height => px(height),
            ],
        ],
    ]
}
fn canvas_with_overlay(
    canvas: &ElRef<HtmlCanvasElement>,
    img: &ElRef<HtmlImageElement>,
    // if canvas should not be subsampled, pass None
    subsampling_pack_option: Option<&SubsamplingPack>,
    zoom: u32,
) -> Node<GMsg> {
    let mut width: u32 = BLOCK_SIZE * zoom;
    let mut height: u32 = BLOCK_SIZE * zoom;

    let mut is_resizable_canvas: bool = false;

    match subsampling_pack_option {
        Some(subsampling_pack) => {
            is_resizable_canvas = true;
            width = width * subsampling_pack.a as u32 / subsampling_pack.j as u32;
            if subsampling_pack.b == 0 {
                height = height / 2;
            }
        }
        None => {}
    }

    canvas_with_overlay_with_w_h(canvas, img, width, height, is_resizable_canvas)
}

fn plot_labeled_div(label: &str, canvas: &ElRef<HtmlCanvasElement>, zoom: u32) -> Node<GMsg> {
    let padding = 10;

    div![
        C!["labeled_canvas_wrapper"],
        label![C!["canvas_label"], &label],
        div![canvas![
            el_ref(&canvas),
            attrs![
                At::Width => px(BLOCK_SIZE * zoom),
                At::Height => px(BLOCK_SIZE * zoom),
            ],
        ],],
        style![
            St::MaxWidth => px(BLOCK_SIZE * zoom + padding * 2),
        ]
    ]
}

pub fn view_settings_sidebar(model: &Model) -> Node<GMsg> {
    div![
        C!["setting_sidebar"],
        input![
            C!["sidebar_activator"],
            attrs! {
                At::Type => "checkbox",
                At::Id => "sidebar_activator",
                At::Name => "sidebar_activator",
            }
        ],
        label![
            C!["sidebar_activator"],
            attrs! {
                At::For => "sidebar_activator"
            },
            span![]
        ],
        div![
            C!["sidebar_settings"],
            label![
                attrs! {
                    At::For => "zoom"
                },
                "Zoom:"
            ],
            input![
                attrs! {
                    At::Type => "range",
                    At::Max => 12,
                    At::Min => 2,
                    At::Value => model.zoom,
                    At::Id => "zoom",
                },
                input_ev("change", |value| {
                    wrap(Msg::ZoomUpdated(value.parse::<u32>().unwrap()))
                })
            ],
            label![
                attrs! {
                    At::For => "subsampling_ratio_select"
                },
                "Subsampling ratio (J:a:b):"
            ],
            select![
                option![
                    "4:4:4",
                    attrs! {
                        At::Value => "4:4:4"
                    }
                ],
                option![
                    "4:2:2",
                    attrs! {
                        At::Value => "4:2:2"
                    }
                ],
                option![
                    "4:1:1",
                    attrs! {
                        At::Value => "4:1:1"
                    }
                ],
                option![
                    "4:4:0",
                    attrs! {
                        At::Value => "4:4:0"
                    }
                ],
                option![
                    "4:2:0",
                    attrs! {
                        At::Value => "4:2:0"
                    }
                ],
                attrs! {
                    At::Id => "subsampling_ratio_select"
                },
                input_ev("change", |value| {
                    let ratio_iter = value.split(':');
                    let ratio_vec = ratio_iter.collect::<Vec<&str>>();
                    let y_ratio = ratio_vec[0].parse::<i8>().unwrap();
                    let cb_ratio = ratio_vec[1].parse::<i8>().unwrap();
                    let cr_ratio = ratio_vec[2].parse::<i8>().unwrap();
                    wrap(Msg::SubsamplingRatioChanged(y_ratio, cb_ratio, cr_ratio))
                })
            ],
            label![
                attrs! {
                    At::For => "quality"
                },
                "Quality:"
            ],
            input![
                attrs! {
                    At::Type => "range",
                    At::Max => 100,
                    At::Value => model.quality,
                    At::Min => 0,
                    At::Id => "quality",
                },
                input_ev("change", |value| {
                    wrap(Msg::QualityUpdated(value.parse::<u8>().unwrap()))
                })
            ],
            table![
                C!["block-content"],
                caption!["Luminance quantization table"],
                (0..8).into_iter().map(|row| {
                    tr![(0..8).into_iter().map(|col| {
                        td![model.scaled_luminance_quant_table[row][col].to_string()]
                    })]
                })
            ],
            table![
                C!["block-content"],
                caption!["Chrominance quantization table"],
                (0..8).into_iter().map(|row| {
                    tr![(0..8).into_iter().map(|col| {
                        td![model.scaled_chrominance_quant_table[row][col].to_string()]
                    })]
                })
            ]
        ]
    ]
}

pub fn view_jpeg_visualization(model: &Model) -> Node<GMsg> {
    div![
        view_settings_sidebar(&model),
        view_image_preview(&model),
        view_ycbcr(&model),
        view_dct_quantized(&model),
        view_ycbcr_recovered(&model),
        view_image_recovered(&model)
    ]
}

pub fn view_file_chooser(model: &Model) -> Node<GMsg> {
    let mut image_divs: Vec<Node<GMsg>> = Vec::<Node<GMsg>>::new();
    for example_image in &model.example_images {
        image_divs.push(tmp(example_image));
    }

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
                    attrs! {
                        At::Type => "file",
                        At::Id => "file",
                        At::Name => "file",
                        At::Accept => "image/*",
                    },
                    ev(Ev::Change, |event| {
                        let file = event
                            .target()
                            .and_then(|target| target.dyn_into::<web_sys::HtmlInputElement>().ok())
                            .and_then(|file_input| file_input.files())
                            .and_then(|file_list| file_list.get(0))
                            .unwrap();
                        wrap(Msg::FileChooserLoadImage(file))
                    }),
                ],
                label![
                    attrs! {
                        At::For => "file"
                    },
                    strong!["Choose a file"],
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
                let file = drag_event
                    .data_transfer()
                    .and_then(|file_input| file_input.files())
                    .and_then(|files| files.get(0))
                    .unwrap();
                wrap(Msg::FileChooserLoadImage(file))
            })
        ],
        div![
            C!["example_images_wrapper"],
            image_divs,
        ],
    ]
}

fn tmp(example_image: &ExampleImage) -> Node<GMsg> {
    div![
        C!["example_image"],
        table![
            tr![
                td![
                    div![
                        C!["example_image_clickable_div"],
                        "TODO - image name",
                    ]
                ]
            ],
            tr![
                td![
                    "TODO - image preview"
                ]
            ],
            tr![
                td![
                    "TODO - width x height".to_owned() + &example_image.tmp_number.to_string()
                ]
            ]
        ]
    ]
}
