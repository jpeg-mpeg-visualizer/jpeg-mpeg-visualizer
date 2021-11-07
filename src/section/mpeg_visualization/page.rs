use super::model::{Model, Msg, State};
use super::view::{view_file_chooser, view_video_player};
use crate::Msg as GMsg;
use gloo_file;
use seed::prelude::*;

pub fn init(_url: Url) -> Option<Model> {
    Some(Model {
        file_chooser_zone_active: false,
        state: State::ChoosingFile,
        hello: 1,
        video_stream_length: 0,
        mpeg1: None,
        renderer: None,
        canvas: ElRef::<_>::default(),
    })
}

pub fn view(model: &Model) -> Node<GMsg> {
    match &model.state {
        State::ChoosingFile => view_file_chooser(model),
        State::DisplayingVideo => view_video_player(model),
    }
}

pub fn wrap(msg: Msg) -> GMsg {
    GMsg::MPEGVisualizationMessage(msg)
}

pub fn update(msg: Msg, model: &mut Model, orders: &mut impl Orders<Msg>) {
    match msg {
        Msg::FileChooserLoadVideo(file) => {
            let file_blob = gloo_file::Blob::from(file);
            orders.perform_cmd(async move {
                let bytes = gloo_file::futures::read_as_bytes(&file_blob).await.unwrap();
                let mut demuxer = super::ts::TSDemuxer::from_raw_bytes(bytes);
                let video_stream = demuxer.parse_packets();
                Msg::VideoLoaded(video_stream)
            });
        }
        Msg::FileChooserDragStarted => model.file_chooser_zone_active = true,
        Msg::FileChooserDragLeave => model.file_chooser_zone_active = false,
        Msg::VideoLoaded(video_stream) => {
            model.state = State::DisplayingVideo;
            let mpeg1 = super::mpeg1::MPEG1::from_bytes(video_stream);
            let renderer = super::renderer::Renderer::new(&model.canvas);
            model.mpeg1 = Some(mpeg1);
            model.renderer = Some(renderer);
        }
        Msg::PlayerClicked => {
            if let Some((mpeg1, renderer)) = model.mpeg1.as_mut().zip(model.renderer.as_mut()) {
                let decoded_frame = mpeg1.decode();
                renderer.render_frame(decoded_frame);
            }
        }
    }
}
