use seed::*;
use seed::prelude::*;

use wasm_bindgen::Clamped;
use web_sys::{HtmlCanvasElement, ImageData};

use super::mpeg1::DecodedFrame;

pub struct Renderer {
    canvas: ElRef<HtmlCanvasElement>,
    width: u16,
    height: u16, 
}

impl Renderer {
    pub fn new(canvas: &ElRef<HtmlCanvasElement>) -> Self {
        Self {
            canvas: canvas.clone(),
            width: 0,
            height: 0
        }
    }
    
    pub fn render_frame(&mut self, frame: DecodedFrame) {
        let canvas = self.canvas.get().unwrap();
        if (frame.width, frame.height) != (self.width, self.height) {
            self.resize(frame.width, frame.height);
        }
        
        log!("frame type: ", frame.picture_type);
        let context = canvas_context_2d(&canvas);
        let mut test_y = frame.y.iter()
            .flat_map(|x| vec![*x, *x, *x, 255])
            .collect::<Vec<_>>();
        let image_data = ImageData::new_with_u8_clamped_array_and_sh(Clamped(&mut test_y), self.width.into(), self.height.into()).unwrap();
        context.put_image_data(&image_data, 0.0, 0.0).unwrap();
    }
    
    fn resize(&mut self, width: u16, height: u16) {
        let canvas = self.canvas.get().unwrap();
        canvas.set_width(width as u32);
        canvas.set_height(height as u32);

        self.width = width;
        self.height = height;
    }
}