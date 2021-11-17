use seed::prelude::web_sys::MouseEvent;
use seed::prelude::*;
use seed::*;

use super::model::{CanvasName, Model, Msg, PreviewCanvasName, State};
use super::page::wrap;
use crate::graphic_helpers::drag_n_drop::*;
use crate::section::jpeg_visualization::model::{PlotName, SubsamplingPack};
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
            canvas_labeled_div_with_overlay(
                "Y QUANTIZED",
                &model.canvas_map.get(&CanvasName::YsQuant).unwrap(),
                &model.overlay_map.get(&CanvasName::YsQuant).unwrap(),
                None,
                model.zoom
            ),
            canvas_labeled_div_with_overlay(
                "CB QUANTIZED",
                &model.canvas_map.get(&CanvasName::CbsQuant).unwrap(),
                &model.overlay_map.get(&CanvasName::CbsQuant).unwrap(),
                Some(&model.subsampling_pack),
                model.zoom
            ),
            canvas_labeled_div_with_overlay(
                "CR QUANTIZED",
                &model.canvas_map.get(&CanvasName::CrsQuant).unwrap(),
                &model.overlay_map.get(&CanvasName::CrsQuant).unwrap(),
                Some(&model.subsampling_pack),
                model.zoom
            ),
            plot_labeled_div(
                "Y QUANTIZED 3D",
                &model.plot_map.get(&PlotName::YsQuant3d).unwrap(),
                model.zoom
            ),
            plot_labeled_div(
                "CB QUANTIZED 3D",
                &model.plot_map.get(&PlotName::CbsQuant3d).unwrap(),
                model.zoom
            ),
            plot_labeled_div(
                "CR QUANTIZED 3D",
                &model.plot_map.get(&PlotName::CrsQuant3d).unwrap(),
                model.zoom
            ),
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
        C!["image_view"],
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
            canvas_labeled_div_with_overlay(
                "DIFFERENCE",
                &model.canvas_map.get(&CanvasName::Difference).unwrap(),
                &model.overlay_map.get(&CanvasName::Difference).unwrap(),
                None,
                model.zoom
            ),
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
    let padding = 10;
    let cloned_canvas_ref = canvas.clone();

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

    div![
        C!["labeled_canvas_wrapper"],
        label![C!["canvas_label"], &label],
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
        ],
        style![
            St::MaxWidth => px(width + padding * 2),
        ]
    ]
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

pub fn view_settings_sidebar(_model: &Model) -> Node<GMsg> {
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
                    At::Default => 7,
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
                    At::Min => 1,
                    At::Id => "quality",
                },
                input_ev("change", |value| {
                    wrap(Msg::QualityUpdated(value.parse::<u8>().unwrap()))
                })
            ],
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
    ]
}
