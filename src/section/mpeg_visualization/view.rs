use crate::graphic_helpers::drag_n_drop::IntoDragEvent;
use crate::section::mpeg_visualization::mpeg1::MacroblockInfoKind;
use crate::Msg as GMsg;
use seed::prelude::*;
use seed::*;
use strum::IntoEnumIterator;
use web_sys::MouseEvent;

use super::model::{ExplainationTab, MacroblockType, Model, Msg};
use super::mpeg1::constants::{PICTURE_TYPE_B, PICTURE_TYPE_INTRA, PICTURE_TYPE_PREDICTIVE};
use super::mpeg1::MacroblockInfo;
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
                        At::Accept => ".ts",
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
                    C![IF!(frame.stats.picture_type == PICTURE_TYPE_INTRA => "-intra")],
                    C![IF!(frame.stats.picture_type == PICTURE_TYPE_PREDICTIVE => "-predictive")],
                    C![IF!(frame.stats.picture_type == PICTURE_TYPE_B => "-bidirectional")],
                    C![IF!(i == model.selected_frame => "-selected")],
                    ev(Ev::Click, move |_| wrap(Msg::FrameChanged(i))),
                    p![(i + 1).to_string()],
                    p![
                        C!["typeletter"],
                        get_frame_type(frame.stats.picture_type, true)
                    ]
                ]
            }),
            IF!(not(model.frames.is_empty()) && model.has_more_frames => {
                div![
                    C!["frame-item", "-more"],
                    ev(Ev::Click, move |_| wrap(Msg::MoreFramesClicked)),
                    "more"
                ]
            })
        ],
        div![
            C!["frame-container"],
            div![
                C!["canvas-container"],
                canvas![C!["canvasindicator"], el_ref(&model.canvas_indicator),],
                canvas![
                    el_ref(&model.canvas),
                    ev(Ev::Click, move |event| {
                        let mouse_event: MouseEvent = event.unchecked_into();
                        wrap(Msg::CanvasClicked(
                            mouse_event.offset_x() as usize,
                            mouse_event.offset_y() as usize,
                        ))
                    })
                ],
            ],
            div![
                C!["frame-sidebar", IF!(model.frames.is_empty() => "-hidden")],
                div![
                    C!["frame-info"],
                    IF!(not(model.frames.is_empty()) => {
                        let decoded_frame = &model.frames[model.selected_frame];
                        let frame = &decoded_frame.frame;
                        vec![
                            h3![format!("Frame #{}", model.selected_frame + 1)],
                            p!["type: ", strong![get_frame_type(decoded_frame.stats.picture_type, false)]],
                            p!["width: ", strong![frame.width.to_string()]],
                            p!["height: ", strong![frame.height.to_string()]],
                            p!["size: ", strong![format!("{:.2} KB", decoded_frame.stats.size as f32 / 1000.0 / 8.0)]],
                            h4!["Additional information"],
                            p!["# of macroblocks: ", strong![decoded_frame.stats.macroblock_count.to_string()]],
                            p!["# of blocks: ", strong![decoded_frame.stats.block_count.to_string()]],
                        ]
                    })
                ],
                div![
                    C!["controls-container"],
                    h3!["Controls"],
                    input![
                        attrs! {At::Type => "checkbox", At::Id => "skipped", At::Checked => model.control_state.skipped.as_at_value()},
                        ev(Ev::Change, move |_| wrap(Msg::ToggleControl(
                            MacroblockType::Skipped
                        )))
                    ],
                    label![attrs! {At::For => "skipped"}, "Show skipped macroblocks"],
                    br![],
                    input![
                        attrs! {At::Type => "checkbox", At::Id => "moved", At::Checked => model.control_state.moved.as_at_value()},
                        ev(Ev::Change, move |_| wrap(Msg::ToggleControl(
                            MacroblockType::Moved
                        )))
                    ],
                    label![attrs! {At::For => "moved"}, "Show moved macroblocks"],
                    br![],
                    input![
                        attrs! {At::Type => "checkbox", At::Id => "intra", At::Checked => model.control_state.intra.as_at_value()},
                        ev(Ev::Change, move |_| wrap(Msg::ToggleControl(
                            MacroblockType::Intra
                        )))
                    ],
                    label![attrs! {At::For => "intra"}, "Show intra macroblocks"],
                ],
                div![
                    C![
                        "macroblock-details",
                        IF!(not(model.selected_macroblock.is_some()) => "-hidden")
                    ],
                    h3!["Macroblock details"],
                    div![IF!(model.selected_macroblock.is_some() => {
                        let macroblock_address = model.selected_macroblock.unwrap();
                        let selected_frame = &model.frames[model.selected_frame];
                        let macroblock_width = (selected_frame.frame.width + 15) / 16;
                        let macroblock_y = macroblock_address / macroblock_width as usize;
                        let macroblock_x = macroblock_address % macroblock_width as usize;
                        let MacroblockInfo {
                            size,
                            encoded_blocks,
                            kind
                        } = &selected_frame.stats.macroblock_info[macroblock_address];
                        vec![
                            p!["x: ", strong![macroblock_x.to_string()], ", y: ", strong![macroblock_y.to_string()]],
                            p!["type: ", strong![format_macroblock_kind(&kind)]],
                            p!["size: ", strong![format!("{} bits", size)]],
                            p![
                                C!["block-container"],
                                "encoded blocks: ",
                                div![
                                    C!["buttonlist"],
                                    ["Y1", "Y2", "Y3", "Y4", "Cr", "Cb"].iter().enumerate().map(|(i, block)| {
                                        button![
                                            C![IF!(model.selected_block == Some(i) => "-selected")],
                                            ev(Ev::Click, move |_| wrap(Msg::BlockSelected(i))),
                                            attrs!{
                                                At::Disabled => encoded_blocks.blocks[i].is_none().as_at_value()
                                            },
                                            block
                                        ]
                                    })
                                ]
                            ],
                            match kind {
                                MacroblockInfoKind::Moved { direction } => p!["direction: ", strong![format!("x: {}, y: {}", direction.0, direction.1)]],
                                _ => seed::empty(),
                            },
                            if let Some(encoded_block) = model.selected_block.and_then(|n| encoded_blocks.blocks[n].as_ref()) {
                                table![
                                    C!["block-content"],
                                    (0..8).into_iter().map(|row| {
                                        tr![
                                            (0..8).into_iter().map(|col| {
                                                td![encoded_block[row * 8 + col].to_string()]
                                            })
                                        ]
                                    })
                                ]
                            } else {
                                empty![]
                            }
                        ]
                    })],
                    {
                        let block_canvas_attrs = attrs! {
                            At::Width => "48",
                            At::Height => "48",
                        };
                        div![
                            C!["block-canvas-list"],
                            div![
                                C!["block"],
                                div![
                                    C!["ys"],
                                    canvas![&block_canvas_attrs, el_ref(&model.canvas_y1)],
                                    canvas![&block_canvas_attrs, el_ref(&model.canvas_y2)],
                                    canvas![&block_canvas_attrs, el_ref(&model.canvas_y3)],
                                    canvas![&block_canvas_attrs, el_ref(&model.canvas_y4)],
                                ],
                                p!["Y"]
                            ],
                            div![
                                C!["block"],
                                canvas![&block_canvas_attrs, el_ref(&model.canvas_cb)],
                                p!["Cb"]
                            ],
                            div![
                                C!["block"],
                                canvas![&block_canvas_attrs, el_ref(&model.canvas_cr)],
                                p!["Cr"]
                            ],
                        ]
                    }
                ],
            ],
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
    match (code, letter) {
        (PICTURE_TYPE_INTRA, true) => "I",
        (PICTURE_TYPE_INTRA, false) => "Intra",
        (PICTURE_TYPE_PREDICTIVE, true) => "P",
        (PICTURE_TYPE_PREDICTIVE, false) => "Predictive",
        (_, true) => "B",
        (_, false) => "Bidirectional",
    }
}

const fn format_macroblock_kind(kind: &MacroblockInfoKind) -> &'static str {
    match kind {
        MacroblockInfoKind::Skipped => "Skipped",
        MacroblockInfoKind::Moved { .. } => "Moved",
        MacroblockInfoKind::Intra => "Intra",
    }
}
