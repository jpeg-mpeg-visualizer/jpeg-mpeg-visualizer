use seed::{prelude::*, *};

use section::*;

mod block;
mod codec;
mod dct;
mod graphic_helpers;
mod image;
mod quant;
mod section;

const BLOCK_SIZE: u32 = 64;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

fn init(url: Url, orders: &mut impl Orders<Msg>) -> Model {
    orders.subscribe(Msg::UrlChanged);
    log("Initialization");
    Model {
        base_url: url.to_hash_base_url(),
        page: Page::init(url),
    }
}

// ------ ------
//     Model
// ------ ------

struct Model {
    base_url: Url,
    page: Page,
}

// ------ Page ------

const JPEG_VISUALIZER: &str = "jpeg-visualizer";
const MPEG_VISUALIZER: &str = "mpeg-visualizer";
const G711_VISUALIZER: &str = "g711-visualizer";

#[allow(clippy::large_enum_variant)]
enum Page {
    Home,
    JPEGVisualizer(jpeg_visualization::model::Model),
    MPEGVisualizer(mpeg_visualization::model::Model),
    G711Visualizer(g711_visualization::model::Model),
    NotFound,
}

impl Page {
    fn init(mut url: Url) -> Self {
        match url.next_hash_path_part() {
            None => Self::Home,
            Some(JPEG_VISUALIZER) => {
                jpeg_visualization::page::init(url).map_or(Self::NotFound, Self::JPEGVisualizer)
            }
            Some(MPEG_VISUALIZER) => {
                mpeg_visualization::page::init(url).map_or(Self::NotFound, Self::MPEGVisualizer)
            }
            Some(G711_VISUALIZER) => {
                g711_visualization::page::init(url).map_or(Self::NotFound, Self::G711Visualizer)
            }
            Some(_) => Self::NotFound,
        }
    }
}

// ------ ------
//     Urls
// ------ ------

struct_urls!();
impl<'a> Urls<'a> {
    pub fn home(self) -> Url {
        self.base_url()
    }
    pub fn jpeg_visualizer(self) -> Url {
        let path = self.base_url().add_hash_path_part(JPEG_VISUALIZER);
        log(&path.to_string());
        path
    }
    pub fn mpeg_visualizer(self) -> Url {
        let path = self.base_url().add_hash_path_part(MPEG_VISUALIZER);
        log(&path.to_string());
        path
    }
    pub fn g711_visualizer(self) -> Url {
        let path = self.base_url().add_hash_path_part(G711_VISUALIZER);
        log(&path.to_string());
        path
    }
}

// ------ ------
//    Update
// ------ ------

pub enum Msg {
    UrlChanged(subs::UrlChanged),
    JPEGVisualizationMessage(jpeg_visualization::model::Msg),
    MPEGVisualizationMessage(mpeg_visualization::model::Msg),
    G711VisualizationMessage(g711_visualization::model::Msg),
}

fn update(msg: Msg, model: &mut Model, orders: &mut impl Orders<Msg>) {
    match msg {
        Msg::UrlChanged(subs::UrlChanged(url)) => {
            match model.page {
                Page::G711Visualizer(ref mut child_model) => {
                    g711_visualization::page::deinit(child_model)
                }
                _ => {}
            }
            model.page = Page::init(url);
        }
        Msg::JPEGVisualizationMessage(child_message) => {
            if let Page::JPEGVisualizer(ref mut child_model) = model.page {
                jpeg_visualization::page::update(
                    child_message,
                    child_model,
                    &mut orders.proxy(Msg::JPEGVisualizationMessage),
                )
            }
        }
        Msg::MPEGVisualizationMessage(child_message) => {
            if let Page::MPEGVisualizer(ref mut child_model) = model.page {
                mpeg_visualization::page::update(
                    child_message,
                    child_model,
                    &mut orders.proxy(Msg::MPEGVisualizationMessage),
                )
            }
        }
        Msg::G711VisualizationMessage(child_message) => {
            if let Page::G711Visualizer(ref mut child_model) = model.page {
                g711_visualization::page::update(
                    child_message,
                    child_model,
                    &mut orders.proxy(Msg::G711VisualizationMessage),
                )
            }
        }
    }
}

fn view(model: &Model) -> impl IntoNodes<Msg> {
    vec![
        header(&model.base_url),
        main![
            C!["content"],
            match &model.page {
                Page::Home => div![
                    C!["select_menu_area"],
                    a![
                        C!["select_menu_button"],
                        attrs! { At::Href => Urls::new(model.base_url.clone()).jpeg_visualizer() },
                        p!["JPEG"]
                    ],
                    a![
                        C!["select_menu_button"],
                        attrs! { At::Href => Urls::new(model.base_url.clone()).mpeg_visualizer() },
                        p!["MPEG-1"]
                    ],
                    a![
                        C!["select_menu_button"],
                        attrs! { At::Href => Urls::new(model.base_url.clone()).g711_visualizer() },
                        p!["G711"]
                    ]
                ],
                Page::JPEGVisualizer(child_model) => jpeg_visualization::page::view(child_model),
                Page::MPEGVisualizer(child_model) => mpeg_visualization::page::view(child_model),
                Page::G711Visualizer(child_model) => g711_visualization::page::view(child_model),
                Page::NotFound => div!["404"],
            }
        ],
    ]
}

fn header(base_url: &Url) -> Node<Msg> {
    nav![
        C!["navbar"],
        ul![li![a![
            attrs! { At::Href => Urls::new(base_url).home() },
            "Compression visualizer",
        ]],]
    ]
}

#[wasm_bindgen(start)]
pub fn main() {
    App::start("app", init, update, view);
}
