use seed::prelude::*;
use seed::{attrs, div, h4, input, label, p, strong, window, C, IF};

use super::model::{Model, Msg};
use super::page::wrap;
use crate::graphic_helpers::drag_n_drop::*;
use crate::Msg as GMsg;

macro_rules! stop_and_prevent {
    { $event:expr } => {
        {
            $event.stop_propagation();
            $event.prevent_default();
        }
     };
}

#[rustfmt::skip]
pub fn view_file_chooser(_: &Model, zone_active: bool) -> Node<GMsg> {
    let preset_audio_divs: Vec<Node<GMsg>> = vec![
        preset_audio_div(
            "music_orig.wav",
            "Music sample",
            "1:31",
        ),
        preset_audio_div(
            "speech_orig.wav",
            "Speech sample",
            "0:11",
        ),
        preset_audio_div(
            "white_noise.mp3",
            "White noise",
            "2:41",
        ),
    ];

    div![
        C!["choose_file_wrapper"],
        div![
            C![
                "drop_area_wrapper",
                IF!(zone_active => "drop_active"),
            ],
            div![
                C!["drop_area"],
                input![
                    C!["drop_file"],
                    attrs! {
                        At::Type => "file",
                        At::Id => "file",
                        At::Name => "file",
                        At::Accept => "audio/*"
                    },
                    ev(Ev::Change, |event| {
                        let file = event
                            .target()
                            .and_then(|target| target.dyn_into::<web_sys::HtmlInputElement>().ok())
                            .and_then(|file_input| file_input.files())
                            .and_then(|file_list| file_list.get(0))
                            .unwrap();
                        wrap(Msg::FileChooserLoadAudio(file))
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
                wrap(Msg::FileChooserLoadAudio(file))
            })
        ],
        div![C!["preset-audio-wrapper"], preset_audio_divs],
    ]
}

fn preset_audio_div(file_name: &str, name: &str, length: &str) -> Node<GMsg> {
    let pathname = window().location().pathname().unwrap();
    let file_path;
    if pathname.ends_with('/') {
        file_path = format!("{}public/preset_audios/{}", pathname, file_name);
    } else {
        file_path = format!("{}/public/preset_audios/{}", pathname, file_name);
    }
    div![
        C!["preset-audio"],
        ev(Ev::Click, |_| wrap(Msg::FileChooserPresetClicked(
            file_path
        ))),
        h4![name],
        p![format!("Length: {}", length)],
    ]
}
