use super::model::{ControlState, MacroblockType, Model, Msg, State};
use super::mpeg1::{DecodedFrame, MPEG1};
use super::view::{view_file_chooser, view_video_player};
use crate::bench::Timer;
use crate::mpeg_visualization::view::view_loading_spinner;
use crate::Msg as GMsg;
use gloo_file;
use seed::prelude::*;

const FRAME_LOAD_COUNT: usize = 50;

pub fn init(_url: Url) -> Option<Model> {
    Some(Model {
        file_chooser_zone_active: false,
        state: State::ChoosingFile,
        hello: 1,
        mpeg1: None,
        renderer: None,
        canvas: ElRef::<_>::default(),
        frames: Vec::new(),
        selected_frame: 0,
        control_state: ControlState {
            skipped: true,
            moved: true,
            intra: true,
        },
        selected_macroblock: None,
        canvas_y1: ElRef::<_>::default(),
        canvas_y2: ElRef::<_>::default(),
        canvas_y3: ElRef::<_>::default(),
        canvas_y4: ElRef::<_>::default(),
        canvas_cb: ElRef::<_>::default(),
        canvas_cr: ElRef::<_>::default(),
        selected_block: None,
        canvas_indicator: ElRef::<_>::default(),
        has_more_frames: true,
        canvas_history_result: ElRef::<_>::default(),
        canvas_history_previous_reference: ElRef::<_>::default(),
        canvas_history_previous_before_diff: ElRef::<_>::default(),
        canvas_history_next_reference: ElRef::<_>::default(),
        canvas_history_next_before_diff: ElRef::<_>::default(),
        canvas_history_interpolated: ElRef::<_>::default(),
    })
}

pub fn view(model: &Model) -> Node<GMsg> {
    match &model.state {
        State::ChoosingFile => view_file_chooser(model),
        State::DisplayingVideo => view_video_player(model),
        State::LoadingSpinnerView => view_loading_spinner(model),
    }
}

pub fn wrap(msg: Msg) -> GMsg {
    GMsg::MPEGVisualizationMessage(msg)
}

pub fn update(msg: Msg, model: &mut Model, orders: &mut impl Orders<Msg>) {
    match msg {
        Msg::FileChooserLoadVideo(file) => {
            model.state = State::LoadingSpinnerView;
            let file_blob = gloo_file::Blob::from(file);
            orders.perform_cmd(async move {
                let loading_timer = Timer::new("MPEG :: loading from file");
                let bytes = gloo_file::futures::read_as_bytes(&file_blob).await.unwrap();
                drop(loading_timer);
                let demuxing_timer = Timer::new("MPEG :: demuxing");
                let mut demuxer = super::ts::TSDemuxer::from_raw_bytes(bytes);
                let video_stream = demuxer.parse_packets();
                drop(demuxing_timer);
                Msg::VideoBytesLoaded(video_stream)
            });
        }
        Msg::FileChooserDragStarted => model.file_chooser_zone_active = true,
        Msg::FileChooserDragLeave => model.file_chooser_zone_active = false,
        Msg::FileChooserPresetClicked(file_path) => {
            model.state = State::LoadingSpinnerView;
            orders.perform_cmd(async move {
                let loading_timer = Timer::new("MPEG :: loading from preset");
                let bytes = Request::new(file_path)
                    .method(Method::Get)
                    .fetch()
                    .await
                    .unwrap()
                    .check_status()
                    .unwrap()
                    .bytes()
                    .await
                    .unwrap();
                drop(loading_timer);

                let demuxing_timer = Timer::new("MPEG :: demuxing");
                let mut demuxer = super::ts::TSDemuxer::from_raw_bytes(bytes);
                let video_stream = demuxer.parse_packets();
                drop(demuxing_timer);
                Msg::VideoBytesLoaded(video_stream)
            });
        }
        Msg::VideoBytesLoaded(video_stream) => {
            let mut mpeg1 = super::mpeg1::MPEG1::from_bytes(video_stream);

            let renderer = super::renderer::Renderer::new(
                &model.canvas,
                &model.canvas_y1,
                &model.canvas_y2,
                &model.canvas_y3,
                &model.canvas_y4,
                &model.canvas_cb,
                &model.canvas_cr,
                &model.canvas_indicator,
                &model.canvas_history_result,
                &model.canvas_history_previous_reference,
                &model.canvas_history_previous_before_diff,
                &model.canvas_history_next_reference,
                &model.canvas_history_next_before_diff,
                &model.canvas_history_interpolated,
            );
            model.renderer = Some(renderer);

            let frames = load_more_frames(&mut mpeg1);
            if frames.len() < FRAME_LOAD_COUNT {
                model.has_more_frames = false;
            }
            model.mpeg1 = Some(mpeg1);

            orders.after_next_render(|_| Msg::PreFrameLoaded(frames));
        }
        Msg::PreFrameLoaded(frames) => {
            model.state = State::DisplayingVideo;
            orders.after_next_render(|_| Msg::FramesLoaded(frames));
        }
        Msg::FramesLoaded(frames) => {
            model.frames.extend(frames);
            model
                .renderer
                .as_mut()
                .unwrap()
                .render_frame(&model.frames[0], &model.control_state);
        }
        Msg::FrameChanged(i) => {
            model.selected_frame = i;
            let frame = &model.frames[i];
            let renderer = model.renderer.as_mut().unwrap();
            renderer.render_frame(frame, &model.control_state);
            if let Some(macroblock_address) = model.selected_macroblock {
                renderer.render_macroblock(frame, macroblock_address);
                renderer.render_history(&model.frames, model.selected_frame, macroblock_address);
            }
        }
        Msg::ToggleControl(macroblock_type) => {
            match macroblock_type {
                MacroblockType::Skipped => {
                    model.control_state.skipped = !model.control_state.skipped
                }
                MacroblockType::Moved => model.control_state.moved = !model.control_state.moved,
                MacroblockType::Intra => model.control_state.intra = !model.control_state.intra,
            };
            model
                .renderer
                .as_mut()
                .unwrap()
                .render_frame(&model.frames[model.selected_frame], &model.control_state);
        }
        Msg::CanvasClicked(mouse_x, mouse_y) => {
            let mb_width = (model.frames[model.selected_frame].frame.width as usize + 15) / 16;
            let macroblock_address = (mouse_y / 16) * mb_width + (mouse_x / 16);
            model.selected_macroblock = Some(macroblock_address);
            let renderer = model.renderer.as_mut().unwrap();
            renderer.render_macroblock(&model.frames[model.selected_frame], macroblock_address);
            renderer.render_history(&model.frames, model.selected_frame, macroblock_address);
        }
        Msg::BlockSelected(index) => {
            model.selected_block = match model.selected_block {
                Some(i) if i == index => None,
                _ => Some(index),
            };
        }
        Msg::MoreFramesClicked => {
            if let Some(mpeg1) = model.mpeg1.as_mut() {
                let frames = load_more_frames(mpeg1);
                if frames.len() < FRAME_LOAD_COUNT {
                    model.has_more_frames = false;
                }
                model.frames.extend(frames);
            }
        }
    }
}

fn load_more_frames(mpeg1: &mut MPEG1) -> Vec<DecodedFrame> {
    let _timer = Timer::new("MPEG :: decoding frames");
    let mut frames = Vec::with_capacity(FRAME_LOAD_COUNT);

    for _ in 0..FRAME_LOAD_COUNT {
        if let Some(frame) = mpeg1.decode() {
            frames.push(frame);
        } else {
            break;
        }
    }

    frames
}
