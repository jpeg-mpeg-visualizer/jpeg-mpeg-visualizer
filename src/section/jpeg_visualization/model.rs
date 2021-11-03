use crate::image;
use seed::prelude::*;
use std::collections::HashMap;
use std::rc::Rc;
use strum_macros::EnumIter;
use web_sys::{HtmlCanvasElement, HtmlImageElement};

pub struct ImagePack {
    pub raw_image: Rc<image::RawImage>,
    pub image_window: image::RawImageWindow,
    pub ycbcr: image::YCbCrImage,
    pub chosen_block_x: f64,
    pub chosen_block_y: f64,
    pub canvases_content: HashMap<CanvasName, Vec<u8>>,
}

pub enum State {
    FileChooser,
    PreImageView,
    ImageView(ImagePack),
}

// ------ ------
//   Messages
// ------ ------

#[derive(Clone)]
pub enum Msg {
    FileChooserLoadImage(web_sys::File),
    FileChooserDragStarted,
    FileChooserDragLeave,
    ImageLoaded(image::RawImage),
    QualityUpdated(u8),
    PreviewCanvasClicked(i32, i32),
    BlockChosen(i32, i32, i32, i32),
}

// ------ ------
//   Canvases
// ------ ------

#[derive(Debug, PartialEq, Eq, Hash, EnumIter, Clone, Copy)]
pub enum CanvasName {
    Ys,
    Cbs,
    Crs,
    YsQuant,
    CbsQuant,
    CrsQuant,
    YsRecovered,
    CbsRecovered,
    CrsRecovered,
    ImageRecovered,
    Difference,
}
#[derive(Debug, PartialEq, Eq, Hash, EnumIter)]
pub enum PreviewCanvasName {
    Original,
    YCbCr,
    YCbCrQuant,
    YCbCrRecovered,
    ForComparison,
}

// ------ ------
//     Model
// ------ ------

pub struct Model {
    pub file_chooser_zone_active: bool,
    pub base_url: Url,
    pub state: State,
    pub original_image_canvas: ElRef<HtmlCanvasElement>,
    pub canvas_map: HashMap<CanvasName, ElRef<HtmlCanvasElement>>,
    pub preview_canvas_map: HashMap<PreviewCanvasName, ElRef<HtmlCanvasElement>>,
    // overlay_image_map and preview_image_map could be one but lack of inheritance makes it at least difficult
    pub original_image_overlay: ElRef<HtmlImageElement>,
    //pub overlay_image_map: HashMap<CanvasName, HtmlImageElement>,
    //pub preview_overlay_image_map: HashMap<CanvasName, HtmlImageElement>,
    pub quality: u8,
}
