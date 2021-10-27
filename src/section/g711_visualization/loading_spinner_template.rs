use seed::prelude::*;
use seed::{div, C};

use super::model::Model;
use crate::Msg as GMsg;

pub fn view_loading_spinner(_: &Model) -> Node<GMsg> {
    div![
        C!["spinner-page"],
        div![
            div![
                C!["dual-spinner"]
            ],
        ]
    ]
}