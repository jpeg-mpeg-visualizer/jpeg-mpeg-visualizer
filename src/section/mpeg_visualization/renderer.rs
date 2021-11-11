use seed::prelude::*;
use seed::*;

use wasm_bindgen::Clamped;
use web_sys::{HtmlCanvasElement, ImageData};

use crate::{
    image::pixel::{self, RGB},
    section::jpeg_visualization::drawing_utils::draw_scaled_image_with_image_data_with_w_h_and_scale,
};

use super::{model::ControlState, mpeg1::DecodedFrame};

pub struct Renderer {
    canvas: ElRef<HtmlCanvasElement>,
    canvas_y1: ElRef<HtmlCanvasElement>,
    canvas_y2: ElRef<HtmlCanvasElement>,
    canvas_y3: ElRef<HtmlCanvasElement>,
    canvas_y4: ElRef<HtmlCanvasElement>,
    canvas_cb: ElRef<HtmlCanvasElement>,
    canvas_cr: ElRef<HtmlCanvasElement>,
    width: u16,
    height: u16,
    rgb_data: Vec<u8>,
    y: Vec<u8>,
    cb: Vec<u8>,
    cr: Vec<u8>,
}

impl Renderer {
    pub fn new(
        canvas: &ElRef<HtmlCanvasElement>,
        canvas_y1: &ElRef<HtmlCanvasElement>,
        canvas_y2: &ElRef<HtmlCanvasElement>,
        canvas_y3: &ElRef<HtmlCanvasElement>,
        canvas_y4: &ElRef<HtmlCanvasElement>,
        canvas_cb: &ElRef<HtmlCanvasElement>,
        canvas_cr: &ElRef<HtmlCanvasElement>,
    ) -> Self {
        Self {
            canvas: canvas.clone(),
            canvas_y1: canvas_y1.clone(),
            canvas_y2: canvas_y2.clone(),
            canvas_y3: canvas_y3.clone(),
            canvas_y4: canvas_y4.clone(),
            canvas_cb: canvas_cb.clone(),
            canvas_cr: canvas_cr.clone(),
            width: 0,
            height: 0,
            rgb_data: Vec::new(),
            y: Vec::new(),
            cb: Vec::new(),
            cr: Vec::new(),
        }
    }

    pub fn render_frame(&mut self, frame: &DecodedFrame, control_state: &ControlState) {
        let canvas = self.canvas.get().unwrap();
        if (frame.width, frame.height) != (self.width, self.height) {
            self.resize(frame.width, frame.height);
        }

        let ControlState {
            skipped,
            moved,
            intra,
        } = control_state;

        let (s_y, s_cb, s_cr) = if *skipped && *moved && *intra {
            (&frame.y, &frame.cb, &frame.cr)
        } else {
            self.y.clear();
            self.cb.clear();
            self.cr.clear();

            self.y.resize(self.width as usize * self.height as usize, 0);
            self.cb
                .resize(self.width as usize * self.height as usize / 4, 0);
            self.cr
                .resize(self.width as usize * self.height as usize / 4, 0);

            for i in 0..self.y.len() {
                if *skipped {
                    self.y[i] += frame.skipped_y[i];
                }
                if *moved {
                    self.y[i] += frame.moved_y[i];
                }
                if *intra {
                    self.y[i] += frame.intra_y[i];
                }
            }

            for i in 0..self.cb.len() {
                if *skipped {
                    self.cb[i] += frame.skipped_cb[i];
                }
                if *moved {
                    self.cb[i] += frame.moved_cb[i];
                }
                if *intra {
                    self.cb[i] += frame.intra_cb[i];
                }
            }

            for i in 0..self.cr.len() {
                if *skipped {
                    self.cr[i] += frame.skipped_cr[i];
                }
                if *moved {
                    self.cr[i] += frame.moved_cr[i];
                }
                if *intra {
                    self.cr[i] += frame.intra_cr[i];
                }
            }

            (&self.y, &self.cb, &self.cr)
        };

        for row in 0..(self.height as usize / 2) {
            for col in 0..(self.width as usize / 2) {
                let y_index = row * 2 * self.width as usize + col * 2;
                let chroma_index = row * (self.width as usize / 2) + col;

                let y1 = s_y[y_index];
                let y2 = s_y[y_index + 1];
                let y3 = s_y[y_index + self.width as usize];
                let y4 = s_y[y_index + self.width as usize + 1];
                let cb = s_cb[chroma_index];
                let cr = s_cr[chroma_index];

                let ycbr1 = crate::image::pixel::YCbCr { y: y1, cr, cb };
                let ycbr2 = crate::image::pixel::YCbCr { y: y2, cr, cb };
                let ycbr3 = crate::image::pixel::YCbCr { y: y3, cr, cb };
                let ycbr4 = crate::image::pixel::YCbCr { y: y4, cr, cb };

                Self::insert_at(&mut self.rgb_data, y_index, ycbr1.to_rgb());
                Self::insert_at(&mut self.rgb_data, y_index + 1, ycbr2.to_rgb());
                Self::insert_at(
                    &mut self.rgb_data,
                    y_index + self.width as usize,
                    ycbr3.to_rgb(),
                );
                Self::insert_at(
                    &mut self.rgb_data,
                    y_index + self.width as usize + 1,
                    ycbr4.to_rgb(),
                );
            }
        }

        let image_data = ImageData::new_with_u8_clamped_array_and_sh(
            Clamped(&mut self.rgb_data),
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
        self.rgb_data
            .resize(width as usize * height as usize * 4, 0);
        self.y.resize(width as usize * height as usize, 0);
        self.cb.resize(width as usize * height as usize / 4, 0);
        self.cr.resize(width as usize * height as usize / 4, 0);
    }

    pub fn render_macroblock(&self, frame: &DecodedFrame, macroblock_index: usize) {
        let mut buffer = [123u8; 64];

        let macroblock_width = (self.width as usize + 15) / 16;
        let y = (macroblock_index / macroblock_width) * 16;
        let x = (macroblock_index % macroblock_width) * 16;
        let chroma_y = y / 2;
        let chroma_x = x / 2;

        self.get_block(x, y, &mut buffer, &frame.y, macroblock_width);
        Self::render_channel(&self.canvas_y1, &buffer, ChannelType::Y);
        self.get_block(x + 8, y, &mut buffer, &frame.y, macroblock_width);
        Self::render_channel(&self.canvas_y2, &buffer, ChannelType::Y);
        self.get_block(x, y + 8, &mut buffer, &frame.y, macroblock_width);
        Self::render_channel(&self.canvas_y3, &buffer, ChannelType::Y);
        self.get_block(x + 8, y + 8, &mut buffer, &frame.y, macroblock_width);
        Self::render_channel(&self.canvas_y4, &buffer, ChannelType::Y);
        self.get_block(
            chroma_x,
            chroma_y,
            &mut buffer,
            &frame.cb,
            macroblock_width / 2,
        );
        Self::render_channel(&self.canvas_cb, &buffer, ChannelType::Cb);
        self.get_block(
            chroma_x,
            chroma_y,
            &mut buffer,
            &frame.cr,
            macroblock_width / 2,
        );
        Self::render_channel(&self.canvas_cr, &buffer, ChannelType::Cr);
    }

    fn render_channel(
        destination: &ElRef<HtmlCanvasElement>,
        data: &[u8; 64],
        channel_type: ChannelType,
    ) {
        let mut buffer = Vec::with_capacity(8 * 8 * 4);
        for value in data {
            let RGB { r, g, b } = match channel_type {
                ChannelType::Y => pixel::YCbCr {
                    y: *value,
                    cb: 128,
                    cr: 128,
                }
                .to_rgb(),
                ChannelType::Cb => pixel::YCbCr {
                    y: 128,
                    cb: *value,
                    cr: 128,
                }
                .to_rgb(),
                ChannelType::Cr => pixel::YCbCr {
                    y: 128,
                    cb: 128,
                    cr: *value,
                }
                .to_rgb(),
            };
            buffer.push(r);
            buffer.push(g);
            buffer.push(b);
            buffer.push(255);
        }

        let image_data =
            ImageData::new_with_u8_clamped_array_and_sh(Clamped(&mut buffer), 8, 8).unwrap();
        let context = canvas_context_2d(&destination.get().unwrap());
        context.set_image_smoothing_enabled(false);
        draw_scaled_image_with_image_data_with_w_h_and_scale(
            destination,
            &image_data,
            8,
            8,
            6.0,
            6.0,
        );
    }

    fn get_block(
        &self,
        x: usize,
        y: usize,
        destination: &mut [u8; 64],
        source: &[u8],
        scan: usize,
    ) {
        for row in 0..8 {
            for col in 0..8 {
                destination[row * 8 + col] = source[(y + row) * scan * 16 + x + col];
            }
        }
    }
}

enum ChannelType {
    Y,
    Cb,
    Cr,
}