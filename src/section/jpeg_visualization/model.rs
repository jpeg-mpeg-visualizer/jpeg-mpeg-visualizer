use crate::image;
use seed::prelude::*;
use std::collections::HashMap;
use std::rc::Rc;
use strum_macros::EnumIter;
use web_sys::HtmlCanvasElement;

pub struct ImagePack {
    pub raw_image: Rc<image::RawImage>,
    pub image_window: image::RawImageWindow,
    pub ycbcr: image::YCbCrImage,
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

    pub quality: u8,
}
