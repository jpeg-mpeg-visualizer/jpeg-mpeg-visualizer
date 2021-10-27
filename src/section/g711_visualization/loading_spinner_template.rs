use web_sys::{DragEvent, Event, MouseEvent};
use seed::prelude::*;
use seed::{div, C, p, strong};

use super::model::{Model, Msg};
use crate::Msg as GMsg;
use crate::graphic_helpers::drag_n_drop::*;
use super::page::wrap;

macro_rules! stop_and_prevent {
    { $event:expr } => {
        {
            $event.stop_propagation();
            $event.prevent_default();
        }
     };
}

pub fn view_loading_spinner(model: &Model) -> Node<GMsg> {
    div![
        C!["spinner-page"],
        div![
            div![
                C!["dual-spinner"]
            ],
        ]
    ]
}