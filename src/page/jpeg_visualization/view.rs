use seed::*;
use seed::prelude::*;
use seed::prelude::web_sys::{DragEvent, Event};

use crate::{Msg as GMsg, image, quant, block};
use super::model::{Model, Msg};
use super::page::wrap;

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

pub fn view_image_preview(model: &Model) -> Node<GMsg> {
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

pub fn view_ycbcr(model: &Model) -> Node<GMsg> {
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

pub fn view_dct_quantized(model: &Model) -> Node<GMsg> {
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

pub fn view_settings_sidebar(model: &Model) -> Node<GMsg> {
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

pub fn view_jpeg_visualization(model: &Model) -> Node<GMsg> {
    div![
        view_settings_sidebar(&model),
        view_image_preview(&model),
        view_ycbcr(&model),
        view_dct_quantized(&model)
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