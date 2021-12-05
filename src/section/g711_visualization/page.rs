use plotters::prelude::*;
use plotters_canvas::CanvasBackend;
use seed::prelude::*;
use seed::struct_urls;
use seed::virtual_dom::ElRef;
use std::ops::Deref;
use std::rc::Rc;
use std::str::FromStr;
use web_sys::{
    window, AudioBufferSourceNode, AudioBufferSourceOptions, AudioContext, HtmlButtonElement,
    HtmlCanvasElement, HtmlDivElement,
};

use super::audio_visualizer_template::view_audio_visualizer;
use super::file_chooser_template::view_file_chooser;
use super::loading_spinner_template::view_loading_spinner;
use super::model::*;
use super::spectrogram::Spectrogram;
use super::utils;
use super::utils::get_upsampled_pcm;
use crate::codec::g711::{alaw, ulaw, SoundDecoder, SoundEncoder};
use crate::section::g711_visualization::model::State::LoadingSpinnerView;
use crate::Msg as GMsg;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

pub fn init(url: Url) -> Option<Model> {
    let base_url = url.to_base_url();
    Some(Model {
        base_url,
        state: State::FileChooser { zone_active: false },

        filename: String::default(),
        bitrate: 0.0,
        length: 0,
        length_8khz: 0,
        pcm_i16: Vec::new(),
        pcm_i16_8khz: Vec::new(),
        compressed_u8_8khz_ulaw: Vec::new(),
        decompressed_pcm_i16_8khz_ulaw: Vec::new(),
        compressed_u8_8khz_alaw: Vec::new(),
        decompressed_pcm_i16_8khz_alaw: Vec::new(),

        player_state: PlayerState::default(),
        playback_source: PlaybackSource::Original,
        compression: Compression::ULaw,
        compression_plot_mode: CompressionChartMode::CurrentCompression,

        original_buffer: None,
        buffer_8khz: None,
        decompressed_buffer_ulaw: Rc::new(None),
        decompressed_buffer_alaw: Rc::new(None),

        pcm_preview: ElRef::<HtmlCanvasElement>::default(),
        compressed_audio_preview: ElRef::<HtmlCanvasElement>::default(),
        spectrogram_canvas: ElRef::<HtmlCanvasElement>::default(),
        progress_bar: ElRef::<HtmlDivElement>::default(),
        current_time: ElRef::<HtmlDivElement>::default(),
        player_wrapper: ElRef::<HtmlDivElement>::default(),

        change_compression_chart_mode: ElRef::<HtmlButtonElement>::default(),
        change_compression: ElRef::<HtmlButtonElement>::default(),
        change_playback: ElRef::<HtmlButtonElement>::default(),

        gain_node: None,
        audio_context: None,
        audio_source: None,

        pcm_i16_ulaw: Vec::new(),
        pcm_i16_alaw: Vec::new(),
        choosen_spectrogram: None,
        cached_spectrogram_original: None,
        cached_spectrogram_alaw: None,
        cached_spectrogram_ulaw: None,
    })
}

#[allow(unused_must_use)]
pub fn deinit(model: &mut Model) {
    if model.audio_context.is_some() {
        model.audio_context.as_mut().unwrap().close().unwrap();
    }
}

// ------ ------
//      View
// ------ ------

pub fn view(model: &Model) -> Node<GMsg> {
    match &model.state {
        State::FileChooser { zone_active } => view_file_chooser(model, *zone_active),
        State::LoadingSpinnerView => view_loading_spinner(model),
        State::PreAudioView => view_audio_visualizer(model),
        State::AudioView => view_audio_visualizer(model),
    }
}

pub fn wrap(msg: Msg) -> GMsg {
    GMsg::G711VisualizationMessage(msg)
}

struct_urls!();
#[allow(dead_code)]
impl<'a> Urls<'a> {
    pub fn base(self) -> Url {
        self.base_url()
    }
}

fn init_audio(model: &mut Model) {
    let context = AudioContext::new().unwrap();

    let gain_node = context.create_gain().unwrap();

    let original_buffer = context
        .create_buffer(1, model.length, model.bitrate)
        .unwrap();

    let buffer_8khz_ulaw = context
        .create_buffer(1, model.length_8khz, 8000_f32)
        .unwrap();

    let buffer_8khz_alaw = context
        .create_buffer(1, model.length_8khz, 8000_f32)
        .unwrap();

    let ulaw = ulaw::ULawCodec {};
    let alaw = alaw::ALawCodec {};

    let compressed_8khz_ulaw = ulaw.encode_frames(&model.pcm_i16_8khz);
    let decompressed_8khz_ulaw = ulaw.decode_frames(&compressed_8khz_ulaw);

    let compressed_8khz_alaw = alaw.encode_frames(&model.pcm_i16_8khz);
    let decompressed_8khz_alaw = alaw.decode_frames(&compressed_8khz_alaw);

    model.compressed_u8_8khz_ulaw = compressed_8khz_ulaw;
    model.decompressed_pcm_i16_8khz_ulaw = decompressed_8khz_ulaw;

    model.compressed_u8_8khz_alaw = compressed_8khz_alaw;
    model.decompressed_pcm_i16_8khz_alaw = decompressed_8khz_alaw;

    let pcm_original_32f = model
        .pcm_i16
        .iter()
        .map(|value| *value as f32 / (i16::MAX as f32 + 1.0))
        .collect::<Vec<f32>>();

    let pcm_ulaw_32f = model
        .decompressed_pcm_i16_8khz_ulaw
        .iter()
        .map(|value| *value as f32 / (i16::MAX as f32 + 1.0))
        .collect::<Vec<f32>>();

    let pcm_alaw_32f = model
        .decompressed_pcm_i16_8khz_alaw
        .iter()
        .map(|value| *value as f32 / (i16::MAX as f32 + 1.0))
        .collect::<Vec<f32>>();

    original_buffer
        .copy_to_channel(&pcm_original_32f, 0)
        .unwrap();
    buffer_8khz_ulaw.copy_to_channel(&pcm_ulaw_32f, 0).unwrap();
    buffer_8khz_alaw.copy_to_channel(&pcm_alaw_32f, 0).unwrap();

    model.player_state = PlayerState::new_with_duration(original_buffer.duration());

    gain_node
        .connect_with_audio_node(&context.destination())
        .unwrap();

    model.original_buffer = Some(original_buffer);
    model.decompressed_buffer_ulaw = Rc::new(Some(buffer_8khz_ulaw));
    model.decompressed_buffer_alaw = Rc::new(Some(buffer_8khz_alaw));

    model.audio_context = Some(context);
    model.gain_node = Some(gain_node);
}

fn init_visualization(model: &mut Model) {
    let pcm_graph_canvas = model.pcm_preview.get().unwrap();
    let g711_graph_canvas = model.compressed_audio_preview.get().unwrap();
    let progress_bar_width = model.progress_bar.get().unwrap().client_width() as u32;
    let window_height = window().unwrap().inner_height().unwrap().as_f64().unwrap() as f32;
    let player_height = model.player_wrapper.get().unwrap().client_height() as f32;

    let visualization_height = ((window_height - player_height) * 0.9 / 2.0) as u32;

    pcm_graph_canvas.set_width(progress_bar_width); //todo: auto resize on screen size change
    pcm_graph_canvas.set_height(visualization_height);

    g711_graph_canvas.set_width(progress_bar_width);
    g711_graph_canvas.set_height(visualization_height);
}

fn draw_frame(model: &Model) {
    let points_count = 240;

    let pcm_graph_canvas = model.pcm_preview.get().unwrap();
    let g711_graph_canvas = model.compressed_audio_preview.get().unwrap();

    let sample_offset = (model.bitrate * model.player_state.position() as f32) as usize;
    let mut sample_end_offset = sample_offset + points_count;

    let scaling_factor = model.bitrate / 8000_f32;
    let sample_8khz_offset = (8000_f32 * model.player_state.position() as f32) as usize;
    let mut sample_8khz_end_offset = (sample_end_offset as f32 / scaling_factor) as usize;

    sample_end_offset = sample_end_offset.min(model.pcm_i16.len());
    sample_8khz_end_offset = sample_8khz_end_offset.min(model.compressed_u8_8khz_ulaw.len());

    let x_axis = (sample_offset as f64 / model.bitrate as f64
        ..sample_end_offset as f64 / model.bitrate as f64)
        .step(0.001);
    let y_axis = (i16::MIN as f64 / 2.0..i16::MAX as f64 / 2.0).step(4096.0);
    let y_axis_8 = (0 as f64..(u8::MAX as u16 + 50) as f64).step(64.0);

    let g711_area = CanvasBackend::with_canvas_object(g711_graph_canvas)
        .unwrap()
        .into_drawing_area();
    let pcm_area = CanvasBackend::with_canvas_object(pcm_graph_canvas)
        .unwrap()
        .into_drawing_area();
    g711_area.fill(&RGBColor(150, 150, 150)).unwrap();
    pcm_area.fill(&RGBColor(150, 150, 150)).unwrap();

    let (compressed_u8_8khz, compression_name, plot_color_index) = match model.compression {
        Compression::ULaw => (&model.compressed_u8_8khz_ulaw, "ULaw", 2),
        Compression::ALaw => (&model.compressed_u8_8khz_alaw, "ALaw", 0),
    };

    let compression_chart_name = match model.compression_plot_mode {
        CompressionChartMode::CurrentCompression => compression_name,
        CompressionChartMode::Both => "ULaw and ALaw",
    };

    let mut compressed_chart = ChartBuilder::on(&g711_area)
        .margin(30)
        .caption(
            format!("G711-{}", compression_chart_name),
            ("sans-serif", 30),
        )
        .set_label_area_size(LabelAreaPosition::Left, (8).percent())
        .set_label_area_size(LabelAreaPosition::Bottom, (4).percent())
        .build_cartesian_2d(x_axis.clone(), y_axis_8)
        .unwrap();

    compressed_chart.configure_mesh().draw().unwrap();

    let compressed_style = Palette99::pick(plot_color_index).mix(0.9).stroke_width(8);

    match model.compression_plot_mode {
        CompressionChartMode::CurrentCompression => {
            compressed_chart
                .draw_series(LineSeries::new(
                    (sample_8khz_offset..sample_8khz_end_offset).map(|x| {
                        (
                            ((x as f32 * scaling_factor) as f64 / model.bitrate as f64),
                            ((i8::from_ne_bytes([compressed_u8_8khz[x]]).to_ne_bytes())[0] as f64),
                        )
                    }),
                    compressed_style,
                ))
                .unwrap()
                .label(compression_name)
                .legend(move |(x, y)| {
                    Rectangle::new(
                        [(x - 5, y - 5), (x + 5, y + 5)],
                        &Palette99::pick(plot_color_index),
                    )
                });
        }
        CompressionChartMode::Both => {
            compressed_chart
                .draw_series(LineSeries::new(
                    (sample_8khz_offset..sample_8khz_end_offset).map(|x| {
                        (
                            ((x as f32 * scaling_factor) as f64 / model.bitrate as f64),
                            ((i8::from_ne_bytes([model.compressed_u8_8khz_ulaw[x]]).to_ne_bytes())
                                [0] as f64),
                        )
                    }),
                    Palette99::pick(2).mix(0.9).stroke_width(8),
                ))
                .unwrap()
                .label("ULaw")
                .legend(move |(x, y)| {
                    Rectangle::new([(x - 5, y - 5), (x + 5, y + 5)], &Palette99::pick(2))
                });
            compressed_chart
                .draw_series(LineSeries::new(
                    (sample_8khz_offset..sample_8khz_end_offset).map(|x| {
                        (
                            ((x as f32 * scaling_factor) as f64 / model.bitrate as f64),
                            ((i8::from_ne_bytes([model.compressed_u8_8khz_alaw[x]]).to_ne_bytes())
                                [0] as f64),
                        )
                    }),
                    Palette99::pick(0).mix(0.9).stroke_width(8),
                ))
                .unwrap()
                .label("Alaw")
                .legend(move |(x, y)| {
                    Rectangle::new([(x - 5, y - 5), (x + 5, y + 5)], &Palette99::pick(0))
                });
        }
    };

    compressed_chart
        .configure_series_labels()
        .border_style(&BLACK)
        .background_style(&WHITE.mix(0.5))
        .draw()
        .unwrap();

    let mut chart = ChartBuilder::on(&pcm_area)
        .margin(30)
        .caption("Original vs Recovered", ("sans-serif", 30))
        .set_label_area_size(LabelAreaPosition::Left, (8).percent())
        .set_label_area_size(LabelAreaPosition::Bottom, (4).percent())
        .build_cartesian_2d(x_axis, y_axis)
        .unwrap();

    chart.configure_mesh().draw().unwrap();

    let original_pcm_style = Palette99::pick(2).mix(0.9).stroke_width(3);
    let recovered_pcm_style = Palette99::pick(0).mix(0.9).stroke_width(3);

    chart
        .draw_series(LineSeries::new(
            (sample_offset..sample_end_offset)
                .map(|x| ((x as f64 / model.bitrate as f64), model.pcm_i16[x] as f64)),
            original_pcm_style,
        ))
        .unwrap()
        .label("Original")
        .legend(move |(x, y)| {
            Rectangle::new([(x - 5, y - 5), (x + 5, y + 5)], &Palette99::pick(2))
        });

    let decompressed_pcm_i16_8khz = match model.compression {
        Compression::ULaw => &model.decompressed_pcm_i16_8khz_ulaw,
        Compression::ALaw => &model.decompressed_pcm_i16_8khz_alaw,
    };

    chart
        .draw_series(LineSeries::new(
            (sample_8khz_offset..sample_8khz_end_offset).map(|x| {
                (
                    (scaling_factor as f64 * x as f64 / model.bitrate as f64),
                    decompressed_pcm_i16_8khz[x] as f64,
                )
            }),
            recovered_pcm_style,
        ))
        .unwrap()
        .label(compression_name)
        .legend(move |(x, y)| {
            Rectangle::new([(x - 5, y - 5), (x + 5, y + 5)], &Palette99::pick(0))
        });

    chart
        .configure_series_labels()
        .border_style(&BLACK)
        .background_style(&WHITE.mix(0.5))
        .draw()
        .unwrap();
}

fn update_view(model: &mut Model) {
    if model.player_state.playing() {
        let current_time = model.audio_context.as_ref().unwrap().current_time();
        let resume_time = model.player_state.resume_time();
        let start_time = model.player_state.start_time();
        let current_diff = current_time - resume_time;
        let mut new_position =
            (resume_time - start_time) + current_diff * model.player_state.speed();

        let duration = model.player_state.duration();

        if new_position >= duration {
            new_position = 0.0;
            model.player_state.toggle_playback_state();
            pause_audio(model);
        }

        model.player_state.set_position(new_position);
    }
}

fn play_audio(model: &mut Model) {
    let context = model.audio_context.as_ref().unwrap();
    let mut options = AudioBufferSourceOptions::new();
    options.playback_rate(model.player_state.speed() as f32);
    let source_node = AudioBufferSourceNode::new_with_options(&context, &options).unwrap();
    source_node
        .connect_with_audio_node(model.gain_node.as_ref().unwrap())
        .unwrap();

    match model.playback_source {
        PlaybackSource::Original => {
            source_node.set_buffer(Some(model.original_buffer.as_ref().unwrap()));
        }
        PlaybackSource::Processed => match model.compression {
            Compression::ULaw => {
                source_node.set_buffer(Some(
                    model.decompressed_buffer_ulaw.deref().as_ref().unwrap(),
                ));
            }
            Compression::ALaw => {
                source_node.set_buffer(Some(
                    model.decompressed_buffer_alaw.deref().as_ref().unwrap(),
                ));
            }
        },
    }

    let current_time = context.current_time();
    let position = model.player_state.position();
    let start_time = current_time - position;
    model.player_state.set_resume_time(current_time);

    model.player_state.set_start_time(start_time);
    source_node
        .start_with_when_and_grain_offset(current_time, position)
        .unwrap();
    model.audio_source = Some(source_node);
    log(&format!(
        "Started playing at {} offset {}. Context time: {}",
        start_time, position, current_time
    ));
}

fn pause_audio(model: &mut Model) {
    model.audio_source.as_ref().unwrap().stop().unwrap();

    let current_time = model.audio_context.as_ref().unwrap().current_time();
    let resume_time = model.player_state.resume_time();
    let start_time = model.player_state.start_time();
    let current_diff = current_time - resume_time;
    let position = (resume_time - start_time) + current_diff * model.player_state.speed();

    model.player_state.set_position(position);
    log(&format!(
        "Stopped at playing at {}. Context time: {}",
        position, current_time
    ));
}

fn seek_audio(model: &mut Model, offset: i32) {
    let timeline_width = model.progress_bar.get().unwrap().client_width() as f64;
    let seek_timeline_position = (offset as f64).clamp(0.0, timeline_width);
    let seek_position = model.player_state.duration() * (seek_timeline_position / timeline_width);

    if model.player_state.playing() {
        pause_audio(model);

        model.player_state.set_position(seek_position);

        play_audio(model);
    } else {
        model.player_state.set_position(seek_position);
    }
}

fn change_playback_speed(model: &mut Model, speed: f64) {
    if model.player_state.playing() {
        pause_audio(model);

        model.player_state.set_speed(speed);

        play_audio(model);
    } else {
        model.player_state.set_speed(speed);
    }
}

fn update_compression_chart_mode_button(model: &Model) {
    let button = model.change_compression_chart_mode.get().unwrap();
    match model.compression_plot_mode {
        CompressionChartMode::CurrentCompression => button.set_inner_text("Switch to Both"),
        CompressionChartMode::Both => button.set_inner_text("Switch to Current Compression"),
    }
}

fn update_compression_button(model: &Model) {
    let button = model.change_compression.get().unwrap();
    match model.compression {
        Compression::ULaw => button.set_inner_text("Switch to ALaw"),
        Compression::ALaw => button.set_inner_text("Switch to ULaw"),
    }
}

fn update_playback_source_button(model: &Model) {
    let button = model.change_playback.get().unwrap();
    match model.playback_source {
        PlaybackSource::Original => button.set_inner_text("Switch playback to G711"),
        PlaybackSource::Processed => button.set_inner_text("Switch playback to Original"),
    }
}

pub fn update(msg: Msg, model: &mut Model, orders: &mut impl Orders<Msg>) {
    match msg {
        Msg::FileChooserLoadAudio(file) => {
            model.filename = file.name();

            let file_blob = gloo_file::Blob::from(file);
            orders.perform_cmd(async move {
                let (raw_sound, bitrate, length, sound_8khz, length_8khz) =
                    utils::load_audio(file_blob).await;
                Msg::PreAudioLoaded(raw_sound, bitrate, length, sound_8khz, length_8khz)
            });

            model.state = LoadingSpinnerView;
        }
        Msg::FileChooserDragStarted => model.state = State::FileChooser { zone_active: true },
        Msg::FileChooserDragLeave => model.state = State::FileChooser { zone_active: false },
        Msg::PreAudioLoaded(audio, bitrate, length, audio_8khz, lenght_8khz) => {
            model.pcm_i16 = audio;
            model.pcm_i16_8khz = audio_8khz;
            model.bitrate = bitrate;
            model.length = length;
            model.length_8khz = lenght_8khz;
            model.state = State::PreAudioView;
            init_audio(model);

            let cloned_ulaw_buffer = model.decompressed_buffer_ulaw.clone();
            let cloned_alaw_buffer = model.decompressed_buffer_alaw.clone();
            orders.perform_cmd(async {
                let upsampled_ulaw = get_upsampled_pcm(cloned_ulaw_buffer, 44100.0).await;
                let upsampled_alaw = get_upsampled_pcm(cloned_alaw_buffer, 44100.0).await;
                Msg::UpsampledDecompressedAudio(upsampled_ulaw, upsampled_alaw)
            });

            orders.after_next_render(|_| Msg::AudioLoaded);
        }
        Msg::AudioLoaded => {
            init_visualization(model);
            update_compression_button(model);
            update_playback_source_button(model);
            draw_frame(model);
            orders.stream(streams::window_event(Ev::Resize, |_| Msg::WindowResized));
            model.state = State::AudioView;
        }

        Msg::WindowResized => {
            init_visualization(model);
        }

        Msg::SwitchCompressionChartMode => {
            model.compression_plot_mode = match model.compression_plot_mode {
                CompressionChartMode::CurrentCompression => CompressionChartMode::Both,
                CompressionChartMode::Both => CompressionChartMode::CurrentCompression,
            };
            update_compression_chart_mode_button(model);
            draw_frame(model);
        }
        Msg::SwitchCompression => {
            model.compression = match model.compression {
                Compression::ULaw => Compression::ALaw,
                Compression::ALaw => Compression::ULaw,
            };
            if model.player_state.playing()
                && matches!(model.playback_source, PlaybackSource::Processed)
            {
                pause_audio(model);
                play_audio(model);
            }
            update_compression_button(model);
            draw_frame(model);
        }
        Msg::SwitchPlayback => {
            model.playback_source = match model.playback_source {
                PlaybackSource::Original => PlaybackSource::Processed,
                PlaybackSource::Processed => PlaybackSource::Original,
            };
            if model.player_state.playing() {
                pause_audio(model);
                play_audio(model);
            }
            update_playback_source_button(model);
            draw_frame(model);
        }

        Msg::TogglePlayer => {
            model.player_state.toggle_playback_state();
            if model.player_state.playing() {
                orders.after_next_render(|data| Msg::FrameUpdate(data));
                play_audio(model);
            } else {
                pause_audio(model);
            }
            draw_frame(model);
        }
        Msg::FrameUpdate(_) => {
            update_view(model);
            if model.player_state.playing() {
                orders.after_next_render(|data| Msg::FrameUpdate(data));
                draw_frame(model);
            }
        }
        Msg::SpeedChanged(speed) => {
            let speed_value = f64::from_str(&speed).unwrap();
            change_playback_speed(model, speed_value)
        }
        Msg::Seek(offset) => {
            seek_audio(model, offset);
            draw_frame(model);
        }
        Msg::SpectogramRequested(choosen_spectrogram) => {
            model.choosen_spectrogram = Some(choosen_spectrogram);
            let spectrogram = match choosen_spectrogram {
                ChoosenSpectrogram::Original => {
                    let pcm = &model.pcm_i16;
                    model
                        .cached_spectrogram_original
                        .get_or_insert_with(|| Spectrogram::new(pcm))
                }
                ChoosenSpectrogram::ULaw => {
                    let pcm = &model.pcm_i16_ulaw;
                    model
                        .cached_spectrogram_ulaw
                        .get_or_insert_with(|| Spectrogram::new(pcm))
                }
                ChoosenSpectrogram::ALaw => {
                    let pcm = &model.pcm_i16_alaw;
                    model
                        .cached_spectrogram_alaw
                        .get_or_insert_with(|| Spectrogram::new(pcm))
                }
            };

            let canvas = model.spectrogram_canvas.get().unwrap();
            spectrogram.draw_spectrogram(canvas);
        }
        Msg::UpsampledDecompressedAudio(ulaw, alaw) => {
            model.pcm_i16_ulaw = ulaw;
            model.pcm_i16_alaw = alaw;
        }
    }
}
