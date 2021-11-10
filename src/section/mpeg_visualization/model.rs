use seed::prelude::ElRef;
use strum_macros::EnumIter;
use web_sys::HtmlCanvasElement;

use super::{
    mpeg1::{DecodedFrame, MPEG1},
    renderer::Renderer,
};

pub enum State {
    ChoosingFile,
    DisplayingVideo,
}

pub struct Model {
    pub state: State,
    pub hello: u8,
    pub file_chooser_zone_active: bool,
    pub mpeg1: Option<MPEG1>,
    pub renderer: Option<Renderer>,
    pub canvas: ElRef<HtmlCanvasElement>,
    pub frames: Vec<DecodedFrame>,
    pub selected_frame: usize,
    pub selected_explaination_tab: ExplainationTab,
}

pub enum Msg {
    FileChooserLoadVideo(web_sys::File),
    FileChooserDragStarted,
    FileChooserDragLeave,
    VideoBytesLoaded(Vec<u8>),
    FramesLoaded(Vec<DecodedFrame>),
    FrameChanged(usize),
    ExplainationTabChanged(ExplainationTab),
}

#[derive(Clone, Copy, PartialEq, EnumIter)]
pub enum ExplainationTab {
    General,
    Intra,
    Predictive,
}

impl ToString for ExplainationTab {
    fn to_string(&self) -> String {
        match self {
            ExplainationTab::General => "General info".into(),
            ExplainationTab::Intra => "Intra frame".into(),
            ExplainationTab::Predictive => "Predictive frame".into(),
        }
    }
}