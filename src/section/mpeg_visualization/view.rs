use crate::graphic_helpers::drag_n_drop::IntoDragEvent;
use crate::Msg as GMsg;
use seed::prelude::*;
use seed::*;
use strum::IntoEnumIterator;

use super::model::{ExplainationTab, MacroblockType, Model, Msg};
use super::mpeg1::constants::{PICTURE_TYPE_INTRA, PICTURE_TYPE_PREDICTIVE};
use super::page::wrap;

macro_rules! stop_and_prevent {
    { $event:expr } => {
        {
            $event.stop_propagation();
            $event.prevent_default();
        }
     };
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
                        wrap(Msg::FileChooserLoadVideo(file))
                    }),
                ],
                label![
                    attrs! {
                        At::For => "file"
                    },
                    strong!["Choose a file"],
                    " or drag it here",
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
                wrap(Msg::FileChooserLoadVideo(file))
            })
        ],
    ]
}

pub fn view_video_player(model: &Model) -> Node<GMsg> {
    div![
        C!["mpeg-container"],
        div![
            C!["frames-accordion"],
            model.frames.iter().enumerate().map(|(i, frame)| {
                div![
                    C!["frame-item"],
                    C![IF!(frame.picture_type == PICTURE_TYPE_INTRA => "-intra")],
                    C![IF!(frame.picture_type == PICTURE_TYPE_PREDICTIVE => "-predictive")],
                    C![IF!(i == model.selected_frame => "-selected")],
                    ev(Ev::Click, move |_| wrap(Msg::FrameChanged(i))),
                    p![(i + 1).to_string()],
                    p![C!["typeletter"], get_frame_type(frame.picture_type, true)]
                ]
            }),
        ],
        div![
            C!["frame-container"],
            canvas![el_ref(&model.canvas)],
            IF!(not(model.frames.is_empty()) => {
                let frame = &model.frames[model.selected_frame];
                div![
                    C!["frame-sidebar"],
                    div![
                        C!["frame-info"],
                        h3![format!("Frame #{}", model.selected_frame + 1)],
                        p!["type: ", strong![get_frame_type(frame.picture_type, false)]],
                        p!["width: ", strong![frame.width.to_string()]],
                        p!["height: ", strong![frame.height.to_string()]],
                        p!["size: ", strong![format!("{:.2} KB", frame.size as f32 / 1000.0 / 8.0)]],
                        h4!["Additional information"],
                        p!["# of macroblocks: ", strong![frame.macroblock_count.to_string()]],
                        p!["# of blocks: ", strong![frame.block_count.to_string()]]
                    ],
                    div![
                        C!["controls-container"],
                        h3!["Controls"],
                        input![
                            attrs!{At::Type => "checkbox", At::Id => "skipped", At::Checked => model.control_state.skipped.as_at_value()},
                            ev(Ev::Change, move |_| wrap(Msg::ToggleControl(MacroblockType::Skipped)))
                        ],
                        label![
                            attrs!{At::For => "skipped"},
                            "Show skipped macroblocks"
                        ],
                        br![],
                        input![
                            attrs!{At::Type => "checkbox", At::Id => "moved", At::Checked => model.control_state.moved.as_at_value()},
                            ev(Ev::Change, move |_| wrap(Msg::ToggleControl(MacroblockType::Moved)))
                        ],
                        label![
                            attrs!{At::For => "moved"},
                            "Show moved macroblocks"
                        ],
                        br![],
                        input![
                            attrs!{At::Type => "checkbox", At::Id => "intra", At::Checked => model.control_state.intra.as_at_value()},
                            ev(Ev::Change, move |_| wrap(Msg::ToggleControl(MacroblockType::Intra)))
                        ],
                        label![
                            attrs!{At::For => "intra"},
                            "Show intra macroblocks"
                        ],
                    ]
                ]
            })
        ],
        IF!(not(model.frames.is_empty()) => {
            div![
                C!["frame-type-explaination"],
                div![
                    C!["tabs"],
                    ExplainationTab::iter().map(|tab| {
                        div![
                            IF!(model.selected_explaination_tab == tab => C!["-selected"]),
                            tab.to_string(),
                            ev(Ev::Click, move |_| wrap(Msg::ExplainationTabChanged(tab)))
                        ]
                    })
                ],
                div![
                    C!["content"],
                    h3!["Lorem ipsum"],
                    "Lorem ipsum dolor sit amet."
                ],
            ]
        })
    ]
}

const fn get_frame_type(code: u8, letter: bool) -> &'static str {
    if code == PICTURE_TYPE_INTRA {
        if letter {
            "I"
        } else {
            "Intra"
        }
    } else {
        if letter {
            "P"
        } else {
            "Predictive"
        }
    }
}
