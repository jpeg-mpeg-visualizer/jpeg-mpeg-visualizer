pub enum State {
    ChoosingFile,
}

pub struct Model {
    pub state: State,
    pub hello: u8,
    pub file_chooser_zone_active: bool,
    pub video_stream_length: usize,
}

pub enum Msg {
    FileChooserLoadVideo(web_sys::File),
    FileChooserDragStarted,
    FileChooserDragLeave,
    VideoLoaded(Vec<u8>),
}
