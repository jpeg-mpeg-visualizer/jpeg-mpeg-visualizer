use seed::prelude::*;
use seed::{attrs, button, canvas, div, h2, option, select, style, unit, C, IF};
use strum::IntoEnumIterator;
use web_sys::MouseEvent;

use super::model::{ChoosenSpectrogram, Model, Msg};
use super::page::wrap;
use crate::Msg as GMsg;

fn audio_time_to_str(time: &f64) -> String {
    let all_seconds = time.ceil() as i32;
    let minutes = all_seconds / 60;
    let seconds = all_seconds % 60;

    format!("{}:{:02}", minutes, seconds)
}

#[rustfmt::skip]
fn audio_player(model: &Model) -> Node<GMsg>{
    let speed: Vec<&str> = vec![
        "1.0",
        "0.75",
        "0.5",
        "0.25",
        "0.1"
    ];

    div![
        C!["audio-player"],
        el_ref(&model.player_wrapper),
        div![
            C!["timeline"],
            el_ref(&model.progress_bar),
            div![
                C!["progress"],
                style!{
                    St::Width => unit!(model.player_state.position()/model.player_state.duration() * 100.0, %),
                },
            ],
            ev(Ev::Click, |event| {
                let mouse_event: MouseEvent = event.unchecked_into();
                wrap(Msg::Seek(mouse_event.offset_x()))
            })
        ],
        div![
            C!["controls"],
            div![
                C!["play-container"],
                div![
                    C![
                        "toggle-play",
                        IF!(!(model.player_state).playing() => "play")
                        IF!((model.player_state).playing() => "pause")
                    ],
                    ev(Ev::Click, |_| {
                        wrap(Msg::TogglePlayer)
                    })
                ]
            ],
            div![
                C!["time"],
                div![
                    el_ref(&model.current_time),
                    C!["current"],
                    audio_time_to_str(&model.player_state.position())
                ],
                div![
                    C!["divider"],
                    "/"
                ],
                div![
                    C!["length"],
                    audio_time_to_str(&model.player_state.duration())
                ]
            ],
            div![
                C!["playback-controls"],
                select![
                    attrs!{At::Name => "speed"},
                    speed.iter().map(|speed_value| {
                        option![
                            attrs!{At::Value => speed_value},
                            speed_value
                        ]
                    }),
                    input_ev(Ev::Change, |val| wrap(Msg::SpeedChanged(val)))
                ],
                button![
                    C!["compression-chart-mode-button"],
                    el_ref(&model.change_compression_chart_mode),
                    "Switch to Both",
                    ev(Ev::Click, |_| {
                        wrap(Msg::SwitchCompressionChartMode)
                    })
                ],
                button![
                    C!["compression-button"],
                    el_ref(&model.change_compression),
                    "Switch to ALaw",
                    ev(Ev::Click, |_| {
                        wrap(Msg::SwitchCompression)
                    })
                ],
                button![
                    C!["playback-button"],
                    el_ref(&model.change_playback),
                    "Switch playback to G711",
                    ev(Ev::Click, |_| {
                        wrap(Msg::SwitchPlayback)
                    })
                ]
            ]
        ]
    ]
}

#[rustfmt::skip]
pub fn view_audio_visualizer(model: &Model) -> Node<GMsg> {
    div![
        C!["audio-visualizer-wrapper"],
        div![
            C!["audio-visualizer-box"],
            div![
                canvas![
                    el_ref(&model.compressed_audio_preview),
                ],
            ],
            div![
                canvas![
                    el_ref(&model.pcm_preview)
                ]
            ],
            div![
                C!["player-wrapper"],
                audio_player(model)
            ],
        ],
        div![
            C!["audio-spectogram"],
            div![
                C!["header"],
                h2!["Spectrogram"],
                div![
                    C!["playback-controls"],
                    ChoosenSpectrogram::iter().map(|choosen_spectrogram| {
                        button![
                            C![IF!(model.choosen_spectrogram == Some(choosen_spectrogram) => "-selected")],
                            ev(Ev::Click, move |_| wrap(Msg::SpectogramRequested(choosen_spectrogram))),
                            format_choosen_spectrogram(choosen_spectrogram),
                        ]
                    }),
                ]
            ],
            canvas![
                C![IF!(model.choosen_spectrogram.is_none() => "-hidden")],
                el_ref(&model.spectrogram_canvas)
            ]
        ]
    ]
}

const fn format_choosen_spectrogram(choosen_spectrogram: ChoosenSpectrogram) -> &'static str {
    match choosen_spectrogram {
        ChoosenSpectrogram::Original => "Original",
        ChoosenSpectrogram::ULaw => "ALaw",
        ChoosenSpectrogram::ALaw => "ULaw",
    }
}
