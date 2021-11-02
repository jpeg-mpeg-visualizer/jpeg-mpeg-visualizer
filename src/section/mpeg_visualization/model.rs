use seed::prelude::ElRef;
use web_sys::HtmlCanvasElement;

use super::{mpeg1::MPEG1, renderer::Renderer};

pub enum State {
    ChoosingFile,
    DisplayingVideo,
}

pub struct Model {
    pub state: State,
    pub hello: u8,
    pub file_chooser_zone_active: bool,
    pub video_stream_length: usize,
    pub mpeg1: Option<MPEG1>,
    pub renderer: Option<Renderer>,
    pub canvas: ElRef<HtmlCanvasElement>,
}

pub enum Msg {
    FileChooserLoadVideo(web_sys::File),
    FileChooserDragStarted,
    FileChooserDragLeave,
    VideoLoaded(Vec<u8>),
    PlayerClicked,
}
