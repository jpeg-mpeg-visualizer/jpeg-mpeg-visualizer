use std::rc::Rc;

use seed::prelude::RenderInfo;
use seed::prelude::*;
use strum_macros::EnumIter;
use web_sys::{
    AudioBuffer, AudioBufferSourceNode, AudioContext, GainNode, HtmlButtonElement,
    HtmlCanvasElement, HtmlDivElement,
};

use super::spectrogram::Spectrogram;

#[derive(Clone)]
pub enum PlayBackState {
    Playing,
    Paused,
}

#[derive(Clone)]
pub struct PlayerState {
    position: f64,
    start_time: f64,
    resume_time: f64,
    duration: f64,
    speed: f64,
    playback_state: PlayBackState,
}

pub enum PlaybackSource {
    Original,
    Processed,
}

pub enum CompressionChartMode {
    CurrentCompression,
    Both,
}

pub enum Compression {
    ULaw,
    ALaw,
}

#[derive(Copy, Clone, EnumIter, PartialEq)]
pub enum ChoosenSpectrogram {
    Original,
    ULaw,
    ALaw,
}

impl Default for PlayerState {
    fn default() -> Self {
        PlayerState {
            position: 0.0,
            start_time: 0.0,
            resume_time: 0.0,
            duration: 0.0,
            speed: 1.0,
            playback_state: PlayBackState::Paused,
        }
    }
}

impl PlayerState {
    pub fn new_with_duration(duration: f64) -> PlayerState {
        PlayerState {
            position: 0.0,
            start_time: 0.0,
            resume_time: 0.0,
            duration,
            speed: 1.0,
            playback_state: PlayBackState::Paused,
        }
    }

    pub fn toggle_playback_state(&mut self) {
        self.playback_state = match self.playback_state {
            PlayBackState::Playing => PlayBackState::Paused,
            PlayBackState::Paused => PlayBackState::Playing,
        }
    }

    #[allow(dead_code)]
    pub fn playback_state(&self) -> PlayBackState {
        self.playback_state.clone()
    }
    pub fn playing(&self) -> bool {
        matches!(self.playback_state, PlayBackState::Playing)
    }

    pub fn start_time(&self) -> f64 {
        self.start_time
    }

    pub fn resume_time(&self) -> f64 {
        self.resume_time
    }

    pub fn position(&self) -> f64 {
        self.position
    }

    pub fn speed(&self) -> f64 {
        self.speed
    }

    pub fn duration(&self) -> f64 {
        self.duration
    }

    pub fn set_position(&mut self, position: f64) {
        self.position = position;
    }

    pub fn set_start_time(&mut self, start_time: f64) {
        self.start_time = start_time;
    }

    pub fn set_resume_time(&mut self, resume_time: f64) {
        self.resume_time = resume_time;
    }

    pub fn set_speed(&mut self, speed: f64) {
        self.speed = speed;
    }
}

pub enum State {
    FileChooser { zone_active: bool },
    LoadingSpinnerView,
    PreAudioView,
    AudioView,
}

// ------ ------
//   Messages
// ------ ------

#[derive(Clone)]
pub enum Msg {
    FileChooserLoadAudio(web_sys::File),
    FileChooserDragStarted,
    FileChooserDragLeave,
    FileChooserPresetClicked(String),
    PreAudioLoaded(Vec<i16>, f32, u32, Vec<i16>, u32),
    AudioLoaded,

    WindowResized,

    TogglePlayer,
    SwitchPlayback,
    SwitchCompression,
    SwitchCompressionChartMode,
    FrameUpdate(RenderInfo),
    Seek(i32),
    SpeedChanged(String),
    SpectogramRequested(ChoosenSpectrogram),
    UpsampledDecompressedAudio(Vec<i16>, Vec<i16>),
}

// ------ ------
//     Model
// ------ ------

pub struct Model {
    pub base_url: Url,
    pub state: State,

    pub filename: String,
    pub bitrate: f32,
    pub length: u32,
    pub length_8khz: u32,

    pub pcm_i16: Vec<i16>,
    pub pcm_i16_8khz: Vec<i16>,

    pub compressed_u8_8khz_ulaw: Vec<u8>,
    pub decompressed_pcm_i16_8khz_ulaw: Vec<i16>,
    pub compressed_u8_8khz_alaw: Vec<u8>,
    pub decompressed_pcm_i16_8khz_alaw: Vec<i16>,

    pub player_state: PlayerState,
    pub playback_source: PlaybackSource,
    pub compression: Compression,
    pub compression_plot_mode: CompressionChartMode,

    pub original_buffer: Option<AudioBuffer>,
    pub buffer_8khz: Option<AudioBuffer>,
    pub decompressed_buffer_ulaw: Rc<Option<AudioBuffer>>,
    pub decompressed_buffer_alaw: Rc<Option<AudioBuffer>>,

    pub pcm_preview: ElRef<HtmlCanvasElement>,
    pub compressed_audio_preview: ElRef<HtmlCanvasElement>,
    pub spectrogram_canvas: ElRef<HtmlCanvasElement>,
    pub progress_bar: ElRef<HtmlDivElement>,
    pub current_time: ElRef<HtmlDivElement>,
    pub player_wrapper: ElRef<HtmlDivElement>,

    pub change_compression_chart_mode: ElRef<HtmlButtonElement>,
    pub change_compression: ElRef<HtmlButtonElement>,
    pub change_playback: ElRef<HtmlButtonElement>,

    pub audio_context: Option<AudioContext>,
    pub gain_node: Option<GainNode>,
    pub audio_source: Option<AudioBufferSourceNode>,

    pub pcm_i16_ulaw: Vec<i16>,
    pub pcm_i16_alaw: Vec<i16>,
    pub choosen_spectrogram: Option<ChoosenSpectrogram>,
    pub cached_spectrogram_original: Option<Spectrogram>,
    pub cached_spectrogram_ulaw: Option<Spectrogram>,
    pub cached_spectrogram_alaw: Option<Spectrogram>,
}
