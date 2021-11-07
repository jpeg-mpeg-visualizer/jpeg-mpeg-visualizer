use seed::prelude::*;
use seed::*;

use wasm_bindgen::Clamped;
use web_sys::{HtmlCanvasElement, ImageData};

use crate::image::pixel::RGB;

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
            height: 0,
        }
    }

    pub fn render_frame(&mut self, frame: DecodedFrame) {
        let canvas = self.canvas.get().unwrap();
        if (frame.width, frame.height) != (self.width, self.height) {
            self.resize(frame.width, frame.height);
        }

        log!("frame type: ", frame.picture_type);
        let mut rgb_data: Vec<u8> = vec![0; self.width as usize * self.height as usize * 4];

        for row in 0..(self.height as usize / 2) {
            for col in 0..(self.width as usize / 2) {
                let y_index = row * 2 * self.width as usize + col * 2;
                let chroma_index = row * (self.width as usize / 2) + col;

                let y1 = frame.y[y_index];
                let y2 = frame.y[y_index + 1];
                let y3 = frame.y[y_index + self.width as usize];
                let y4 = frame.y[y_index + self.width as usize + 1];
                let cb = frame.cb[chroma_index];
                let cr = frame.cr[chroma_index];

                let ycbr1 = crate::image::pixel::YCbCr { y: y1, cr, cb };
                let ycbr2 = crate::image::pixel::YCbCr { y: y2, cr, cb };
                let ycbr3 = crate::image::pixel::YCbCr { y: y3, cr, cb };
                let ycbr4 = crate::image::pixel::YCbCr { y: y4, cr, cb };

                Self::insert_at(&mut rgb_data, y_index, ycbr1.to_rgb());
                Self::insert_at(&mut rgb_data, y_index + 1, ycbr2.to_rgb());
                Self::insert_at(&mut rgb_data, y_index + self.width as usize, ycbr3.to_rgb());
                Self::insert_at(
                    &mut rgb_data,
                    y_index + self.width as usize + 1,
                    ycbr4.to_rgb(),
                );
            }
        }

        let image_data = ImageData::new_with_u8_clamped_array_and_sh(
            Clamped(&mut rgb_data),
            self.width.into(),
            self.height.into(),
        )
        .unwrap();
        let context = canvas_context_2d(&canvas);
        context.put_image_data(&image_data, 0.0, 0.0).unwrap();
    }

    fn insert_at(vec: &mut Vec<u8>, index: usize, rgb: RGB) {
        let RGB { r, g, b } = rgb;
        vec[index * 4] = r;
        vec[index * 4 + 1] = g;
        vec[index * 4 + 2] = b;
        vec[index * 4 + 3] = 255;
    }

    fn resize(&mut self, width: u16, height: u16) {
        let canvas = self.canvas.get().unwrap();
        canvas.set_width(width as u32);
        canvas.set_height(height as u32);

        self.width = width;
        self.height = height;
    }
}
