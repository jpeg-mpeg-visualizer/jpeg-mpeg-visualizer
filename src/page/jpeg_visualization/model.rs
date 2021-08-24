use crate::image;
use seed::prelude::*;
use web_sys::HtmlCanvasElement;
use web_sys::HtmlDivElement;

pub struct ImagePack {
    pub raw_image: image::RawImage,
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
//     Model
// ------ ------

pub struct Model {
    pub file_chooser_zone_active: bool,
    pub base_url: Url,
    pub state: State,
    pub original_canvas_preview: ElRef<HtmlCanvasElement>,
    pub original_canvas: ElRef<HtmlCanvasElement>,
    pub original_canvas_scrollable_div_wrapper: ElRef<HtmlDivElement>,
    pub ys_canvas: ElRef<HtmlCanvasElement>,
    pub cbs_canvas: ElRef<HtmlCanvasElement>,
    pub crs_canvas: ElRef<HtmlCanvasElement>,
    pub ys_quant_canvas: ElRef<HtmlCanvasElement>,
    pub cbs_quant_canvas: ElRef<HtmlCanvasElement>,
    pub crs_quant_canvas: ElRef<HtmlCanvasElement>,

    pub quality: u8,
}
