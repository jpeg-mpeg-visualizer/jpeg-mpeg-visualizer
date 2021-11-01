use super::model::{Model, Msg, State};
use crate::Msg as GMsg;
use gloo_file;
use seed::prelude::web_sys::{DragEvent, Event};
use seed::prelude::*;
use seed::*;

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

trait IntoDragEvent {
    fn into_drag_event(self) -> DragEvent;
}

impl IntoDragEvent for Event {
    fn into_drag_event(self) -> DragEvent {
        self.dyn_into::<web_sys::DragEvent>()
            .expect("cannot cast given event into DragEvent")
    }
}

fn wrap(msg: Msg) -> GMsg {
    GMsg::MPEGVisualizationMessage(msg)
}

macro_rules! stop_and_prevent {
    { $event:expr } => {
        {
            $event.stop_propagation();
            $event.prevent_default();
        }
     };
}

pub fn view_file_chooser(model: &Model) -> Node<GMsg> {
    div![
        C!["choose_file_wrapper"],
        div![
            C![
                "drop_area_wrapper",
                IF!(model.file_chooser_zone_active => "drop_active"),
            ],
            div![
                C!["drop_area"],
                input![
                    C!["drop_file"],
                    attrs! {
                        At::Type => "file",
                        At::Id => "file",
                        At::Name => "file",
                    },
                    ev(Ev::Change, |event| {
                        let file = event
                            .target()
                            .and_then(|target| target.dyn_into::<web_sys::HtmlInputElement>().ok())
                            .and_then(|file_input| file_input.files())
                            .and_then(|file_list| file_list.get(0))
                            .unwrap();
                        wrap(Msg::FileChooserLoadVideo(file))
                    }),
                ],
                label![
                    attrs! {
                        At::For => "file"
                    },
                    strong!["Choose a file"],
                    format!(" or drag it here {:?}", model.video_stream_length),
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
                wrap(Msg::FileChooserLoadVideo(file))
            })
        ],
    ]
}

pub fn view_video_player(model: &Model) -> Node<GMsg> {
    div![
        canvas![
            el_ref(&model.canvas),
            ev(Ev::Click, |event| {
                wrap(Msg::PlayerClicked)
            })
        ]
    ]
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
