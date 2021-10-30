use seed::prelude::web_sys::MouseEvent;
use seed::prelude::*;
use seed::*;

use super::model::{CanvasName, PreviewCanvasName, Model, Msg, State};
use super::page::wrap;
use crate::graphic_helpers::drag_n_drop::*;
use crate::{Msg as GMsg, BLOCK_SIZE, ZOOM};
use web_sys::HtmlCanvasElement;

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
            canvas![
                el_ref(&model.original_image_canvas),
                attrs![
                    At::Width => px(BLOCK_SIZE * ZOOM),
                    At::Height => px(BLOCK_SIZE * ZOOM),
                ],
                ev(Ev::Click, |event| {
                    let mouse_event: MouseEvent = event.unchecked_into();
                    wrap(Msg::PreviewCanvasClicked(mouse_event.x(), mouse_event.y()))
                })
            ],
            canvas_labeled_div("INPUT", &model.preview_canvas_map.get(&PreviewCanvasName::Original).unwrap()),
        ]
    ]
}

pub fn view_ycbcr(model: &Model) -> Node<GMsg> {
    div![
        C!["image_view"],
        details![
            summary!["YCbCr"],
            canvas_labeled_div("INPUT", &model.preview_canvas_map.get(&PreviewCanvasName::YCbCr).unwrap()),
            canvas_labeled_div(" Y ", &model.canvas_map.get(&CanvasName::Ys).unwrap()),
            canvas_labeled_div("CB", &model.canvas_map.get(&CanvasName::Cbs).unwrap()),
            canvas_labeled_div("CR", &model.canvas_map.get(&CanvasName::Crs).unwrap()),
        ]
    ]
}

pub fn view_dct_quantized(model: &Model) -> Node<GMsg> {
    div![
        C!["image_view"],
        details![
            summary!["DCT Quantized"],
            canvas_labeled_div("INPUT", &model.preview_canvas_map.get(&PreviewCanvasName::YCbCrQuant).unwrap()),
            canvas_labeled_div("Y QUANTIZED", &model.canvas_map.get(&CanvasName::YsQuant).unwrap()),
            canvas_labeled_div("CB QUANTIZED", &model.canvas_map.get(&CanvasName::CbsQuant).unwrap()),
            canvas_labeled_div("CR QUANTIZED", &model.canvas_map.get(&CanvasName::CrsQuant).unwrap()),
        ]
    ]
}

fn view_ycbcr_recovered(model: &Model) -> Node<GMsg> {
    div![
        C!["image_view"],
        details![
            summary!["YCbCr recovered from quantized DCT"],
            canvas_labeled_div("INPUT", &model.preview_canvas_map.get(&PreviewCanvasName::YCbCrRecovered).unwrap()),
            canvas_labeled_div("Y RECOVERED", &model.canvas_map.get(&CanvasName::YsRecovered).unwrap()),
            canvas_labeled_div("CB RECOVERED", &model.canvas_map.get(&CanvasName::CbsRecovered).unwrap()),
            canvas_labeled_div("CR RECOVERED", &model.canvas_map.get(&CanvasName::CrsRecovered).unwrap()),
        ]
    ]
}

fn view_image_recovered(model: &Model) -> Node<GMsg> {
    div![
        C!["image_view"],
        details![
            summary!["Recovered image and comparison"],
            canvas_labeled_div("INPUT", &model.preview_canvas_map.get(&PreviewCanvasName::ForComparison).unwrap()),
            canvas_labeled_div("OUTPUT", &model.canvas_map.get(&CanvasName::ImageRecovered).unwrap()),
            canvas_labeled_div("DIFFERENCE", &model.canvas_map.get(&CanvasName::Difference).unwrap()),
        ]
    ]
}

fn canvas_labeled_div(label: &str, canvas: &ElRef<HtmlCanvasElement>) -> Node<GMsg> {
    let padding = 10;
    let cloned_canvas = canvas.clone();
    div![
        C!["labeled_canvas_wrapper"],
        label![C!["canvas_label"], &label],
        canvas![
            el_ref(&canvas),
            attrs![
                At::Width => px(BLOCK_SIZE * ZOOM),
                At::Height => px(BLOCK_SIZE * ZOOM),
            ],
            ev(Ev::Click, move|event| {
                let mouse_event: MouseEvent = event.unchecked_into();
                let canvas_rect = cloned_canvas
                    .get()
                    .unwrap()
                    .get_bounding_client_rect();
                wrap(Msg::BlockChosen(mouse_event.x(), mouse_event.y(), canvas_rect.left() as i32, canvas_rect.top() as i32))
            })
        ],
        style![
            St::MaxWidth => px(BLOCK_SIZE * ZOOM + padding * 2),
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
