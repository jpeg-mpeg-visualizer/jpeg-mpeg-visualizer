use seed::prelude::*;
use seed::prelude::RenderInfo;
use web_sys::{
    HtmlCanvasElement, AudioBuffer,
    HtmlDivElement, AudioContext,
    AudioBufferSourceNode, GainNode,
    HtmlButtonElement
};

#[derive(Clone)]
pub enum PlayBackState {
    Playing,
    Paused
}

#[derive(Clone)]
pub struct PlayerState {
    position: f64,
    start_time: f64,
    duration: f64,
    playback_state: PlayBackState,
}

pub enum PlaybackSource {
    Original,
    Processed
}

pub enum Compression {
    ULaw,
    ALaw
}

impl Default for PlayerState {
    fn default() -> Self {
        PlayerState {
            position: 0.0,
            start_time: 0.0,
            duration: 0.0,
            playback_state: PlayBackState::Paused
        }
    }
}

impl PlayerState {
    pub fn new_with_duration(duration: f64) -> PlayerState {
        PlayerState {
            position: 0.0,
            start_time: 0.0,
            duration,
            playback_state: PlayBackState::Paused
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

    pub fn position(&self) -> f64 {
        self.position
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
}

pub enum State {
    FileChooser {zone_active: bool},
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
    PreAudioLoaded(Vec<i16>, f32, u32, Vec<i16>, u32),
    AudioLoaded,

    WindowResized,

    TogglePlayer,
    SwitchPlayback,
    SwitchCompression,
    FrameUpdate(RenderInfo),
    Seek(i32)
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

    pub original_buffer: Option<AudioBuffer>,
    pub buffer_8khz: Option<AudioBuffer>,
    pub decompressed_buffer_ulaw: Option<AudioBuffer>,
    pub decompressed_buffer_alaw: Option<AudioBuffer>,

    pub pcm_preview: ElRef<HtmlCanvasElement>,
    pub compressed_audio_preview: ElRef<HtmlCanvasElement>,
    pub progress_bar: ElRef<HtmlDivElement>,
    pub current_time: ElRef<HtmlDivElement>,
    pub player_wrapper: ElRef<HtmlDivElement>,

    pub change_compression: ElRef<HtmlButtonElement>,
    pub change_playback: ElRef<HtmlButtonElement>,

    pub audio_context: Option<AudioContext>,
    pub gain_node: Option<GainNode>,
    pub audio_source: Option<AudioBufferSourceNode>
}