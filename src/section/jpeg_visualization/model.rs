use crate::block::BlockMatrix;
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

    pub plot_data: HashMap<PlotName, BlockMatrix>,

    pub chosen_block_x: f64,
    pub chosen_block_y: f64,
}

pub struct SubsamplingPack {
    pub j: i8,
    pub a: i8,
    pub b: i8,
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
    FileChooserPresetClicked(String),
    ImageLoaded(image::RawImage),
    QualityUpdated(u8),
    ZoomUpdated(u32),
    PostZoomUpdated,
    PreviewCanvasClicked(i32, i32),
    BlockChosen(i32, i32, i32, i32, bool),
    SubsamplingRatioChanged(i8, i8, i8),
    PostSubsamplingRatioChanged,
    DiffInfoDisplayChanged,
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
pub fn is_canvas_subsampled(canvas_name: &CanvasName) -> bool {
    return match canvas_name {
        CanvasName::Cbs
        | CanvasName::Crs
        | CanvasName::CbsQuant
        | CanvasName::CrsQuant
        | CanvasName::CbsRecovered
        | CanvasName::CrsRecovered => true,
        _ => false,
    };
}

#[derive(PartialEq, Eq, Hash, EnumIter, Clone, Copy)]
pub enum PlotName {
    YsQuant3d,
    CbsQuant3d,
    CrsQuant3d,
}

#[derive(Debug, PartialEq, Eq, Hash, EnumIter, Clone, Copy)]
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
    pub plot_map: HashMap<PlotName, ElRef<HtmlCanvasElement>>,
    pub chosen_block_plot_map: HashMap<PlotName, ElRef<HtmlCanvasElement>>,

    pub original_image_overlay: ElRef<HtmlImageElement>,
    // overlay_map and preview_overlay_map could be one but lack of inheritance makes it at least difficult
    pub overlay_map: HashMap<CanvasName, ElRef<HtmlImageElement>>,
    pub preview_overlay_map: HashMap<PreviewCanvasName, ElRef<HtmlImageElement>>,

    pub quality: u8,
    pub zoom: u32,
    pub is_diff_info_shown: bool,
    pub subsampling_pack: SubsamplingPack,

    pub scaled_luminance_quant_table: [[u8; 8]; 8],
    pub scaled_chrominance_quant_table: [[u8; 8]; 8],
}
