use seed::prelude::ElRef;
use web_sys::HtmlCanvasElement;

use super::{
    mpeg1::{DecodedFrame, MPEG1},
    renderer::Renderer,
};

pub enum State {
    ChoosingFile,
    DisplayingVideo,
    LoadingSpinnerView,
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
    pub control_state: ControlState,
    pub selected_macroblock: Option<usize>,
    pub canvas_y1: ElRef<HtmlCanvasElement>,
    pub canvas_y2: ElRef<HtmlCanvasElement>,
    pub canvas_y3: ElRef<HtmlCanvasElement>,
    pub canvas_y4: ElRef<HtmlCanvasElement>,
    pub canvas_cb: ElRef<HtmlCanvasElement>,
    pub canvas_cr: ElRef<HtmlCanvasElement>,
    pub selected_block: Option<usize>,
    pub canvas_indicator: ElRef<HtmlCanvasElement>,
    pub has_more_frames: bool,
    pub canvas_history_result: ElRef<HtmlCanvasElement>,
    pub canvas_history_previous_reference: ElRef<HtmlCanvasElement>,
    pub canvas_history_previous_before_diff: ElRef<HtmlCanvasElement>,
    pub canvas_history_next_reference: ElRef<HtmlCanvasElement>,
    pub canvas_history_next_before_diff: ElRef<HtmlCanvasElement>,
    pub canvas_history_interpolated: ElRef<HtmlCanvasElement>,
}

pub enum Msg {
    FileChooserLoadVideo(web_sys::File),
    FileChooserDragStarted,
    FileChooserDragLeave,
    FileChooserPresetClicked(String),
    VideoBytesLoaded(Vec<u8>),
    PreFrameLoaded(Vec<DecodedFrame>),
    FramesLoaded(Vec<DecodedFrame>),
    FrameChanged(usize),
    ToggleControl(MacroblockType),
    CanvasClicked(usize, usize),
    BlockSelected(usize),
    MoreFramesClicked,
}

pub struct ControlState {
    pub skipped: bool,
    pub moved: bool,
    pub intra: bool,
}

pub enum MacroblockType {
    Skipped,
    Moved,
    Intra,
}
