use crate::image;
use seed::prelude::*;
use std::rc::Rc;
use web_sys::HtmlCanvasElement;
use web_sys::HtmlDivElement;
use std::collections::HashMap;
use strum_macros::EnumIter;

pub struct ImagePack {
    pub raw_image: Rc<image::RawImage>,
    pub image_window: image::RawImageWindow,
    pub start_x: u32,
    pub start_y: u32,
    pub ycbcr: image::YCbCrImage,
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
    BlockChosen(i32, i32),
}

// ------ ------
//   Canvases
// ------ ------

#[derive(Debug, PartialEq, Eq, Hash, EnumIter)]
pub enum CanvasName {
    Original,
    OriginalPreview,
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
}

// ------ ------
//     Model
// ------ ------

pub struct Model {
    pub file_chooser_zone_active: bool,
    pub base_url: Url,
    pub state: State,
    /*pub original_canvas_preview: ElRef<HtmlCanvasElement>,
    pub original_canvas: ElRef<HtmlCanvasElement>,*/
    pub original_canvas_scrollable_div_wrapper: ElRef<HtmlDivElement>,
    /*pub ys_canvas: ElRef<HtmlCanvasElement>,
    pub cbs_canvas: ElRef<HtmlCanvasElement>,
    pub crs_canvas: ElRef<HtmlCanvasElement>,
    pub ys_quant_canvas: ElRef<HtmlCanvasElement>,
    pub cbs_quant_canvas: ElRef<HtmlCanvasElement>,
    pub crs_quant_canvas: ElRef<HtmlCanvasElement>,
    pub ys_recovered_canvas: ElRef<HtmlCanvasElement>,
    pub cbs_recovered_canvas: ElRef<HtmlCanvasElement>,
    pub crs_recovered_canvas: ElRef<HtmlCanvasElement>,
    pub image_recovered_canvas: ElRef<HtmlCanvasElement>,*/
    pub canvas_map: HashMap<CanvasName, ElRef<HtmlCanvasElement>>,

    pub quality: u8,
}
