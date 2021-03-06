use seed::prelude::*;
use seed::*;

use wasm_bindgen::Clamped;
use web_sys::{HtmlCanvasElement, ImageData};

use crate::{
    image::pixel::{self, RGB},
    section::{
        jpeg_visualization::drawing_utils::draw_scaled_image_with_image_data_with_w_h_and_scale,
        mpeg_visualization::mpeg1::constants,
    },
};

use super::{
    model::ControlState,
    mpeg1::{DecodedFrame, MacroblockContent, MacroblockInfoKind, VideoFrame},
};

pub struct Renderer {
    canvas: ElRef<HtmlCanvasElement>,
    canvas_y1: ElRef<HtmlCanvasElement>,
    canvas_y2: ElRef<HtmlCanvasElement>,
    canvas_y3: ElRef<HtmlCanvasElement>,
    canvas_y4: ElRef<HtmlCanvasElement>,
    canvas_cb: ElRef<HtmlCanvasElement>,
    canvas_cr: ElRef<HtmlCanvasElement>,
    canvas_indicator: ElRef<HtmlCanvasElement>,
    canvas_history_result: ElRef<HtmlCanvasElement>,
    canvas_history_previous_reference: ElRef<HtmlCanvasElement>,
    canvas_history_previous_before_diff: ElRef<HtmlCanvasElement>,
    canvas_history_next_reference: ElRef<HtmlCanvasElement>,
    canvas_history_next_before_diff: ElRef<HtmlCanvasElement>,
    canvas_history_interpolated: ElRef<HtmlCanvasElement>,
    width: u16,
    height: u16,
    rgb_data: Vec<u8>,
    y: Vec<u8>,
    cb: Vec<u8>,
    cr: Vec<u8>,
}

impl Renderer {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        canvas: &ElRef<HtmlCanvasElement>,
        canvas_y1: &ElRef<HtmlCanvasElement>,
        canvas_y2: &ElRef<HtmlCanvasElement>,
        canvas_y3: &ElRef<HtmlCanvasElement>,
        canvas_y4: &ElRef<HtmlCanvasElement>,
        canvas_cb: &ElRef<HtmlCanvasElement>,
        canvas_cr: &ElRef<HtmlCanvasElement>,
        canvas_indicator: &ElRef<HtmlCanvasElement>,
        canvas_history_result: &ElRef<HtmlCanvasElement>,
        canvas_history_previous_reference: &ElRef<HtmlCanvasElement>,
        canvas_history_previous_before_diff: &ElRef<HtmlCanvasElement>,
        canvas_history_next_reference: &ElRef<HtmlCanvasElement>,
        canvas_history_next_before_diff: &ElRef<HtmlCanvasElement>,
        canvas_history_interpolated: &ElRef<HtmlCanvasElement>,
    ) -> Self {
        Self {
            canvas: canvas.clone(),
            canvas_y1: canvas_y1.clone(),
            canvas_y2: canvas_y2.clone(),
            canvas_y3: canvas_y3.clone(),
            canvas_y4: canvas_y4.clone(),
            canvas_cb: canvas_cb.clone(),
            canvas_cr: canvas_cr.clone(),
            canvas_indicator: canvas_indicator.clone(),
            canvas_history_result: canvas_history_result.clone(),
            canvas_history_previous_reference: canvas_history_previous_reference.clone(),
            canvas_history_previous_before_diff: canvas_history_previous_before_diff.clone(),
            canvas_history_next_reference: canvas_history_next_reference.clone(),
            canvas_history_next_before_diff: canvas_history_next_before_diff.clone(),
            canvas_history_interpolated: canvas_history_interpolated.clone(),
            width: 0,
            height: 0,
            rgb_data: Vec::new(),
            y: Vec::new(),
            cb: Vec::new(),
            cr: Vec::new(),
        }
    }

    pub fn render_frame(&mut self, decoded_frame: &DecodedFrame, control_state: &ControlState) {
        let (frame, stats) = (&decoded_frame.frame, &decoded_frame.stats);
        let canvas = self.canvas.get().unwrap();
        if (frame.width, frame.height) != (self.width, self.height) {
            self.resize(frame.width, frame.height);
        }

        let &ControlState {
            skipped,
            moved,
            intra,
        } = control_state;

        self.rgb_data.clear();
        self.rgb_data
            .resize(self.width as usize * self.height as usize * 4, 0);
        let mb_width = (self.width as usize + 15) / 16;

        for row in 0..(self.height as usize / 2) {
            for col in 0..(self.width as usize / 2) {
                let macroblock_address = (row / 8) * mb_width + (col / 8);

                match stats.macroblock_info[macroblock_address].kind {
                    MacroblockInfoKind::Skipped if !skipped => continue,
                    MacroblockInfoKind::Interpolated { .. } | MacroblockInfoKind::Moved { .. }
                        if !moved =>
                    {
                        continue
                    }
                    MacroblockInfoKind::Intra if !intra => continue,
                    _ => {}
                };

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

        let canvas_indicator = self.canvas_indicator.get().unwrap();
        canvas_indicator.set_width(width as u32);
        canvas_indicator.set_height(height as u32);

        self.width = width;
        self.height = height;
        self.rgb_data
            .resize(width as usize * height as usize * 4, 0);
        self.y.resize(width as usize * height as usize, 0);
        self.cb.resize(width as usize * height as usize / 4, 0);
        self.cr.resize(width as usize * height as usize / 4, 0);
    }

    pub fn render_macroblock(&self, decoded_frame: &DecodedFrame, macroblock_index: usize) {
        let mut buffer = [123u8; 64];
        let frame = &decoded_frame.frame;

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

        self.draw_indicator(x, y);
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

    fn draw_indicator(&self, x: usize, y: usize) {
        let canvas_indicator = self.canvas_indicator.get().unwrap();
        let context = canvas_context_2d(&canvas_indicator);
        const LINE_WIDTH: f64 = 4.0;

        context.clear_rect(0.0, 0.0, self.width.into(), self.height.into());
        context.set_line_width(LINE_WIDTH);
        context.stroke_rect(
            x as f64 - LINE_WIDTH / 2.0,
            y as f64 - LINE_WIDTH / 2.0,
            16.0 + LINE_WIDTH,
            16.0 + LINE_WIDTH,
        );
    }

    pub fn render_history(
        &self,
        frames: &Vec<DecodedFrame>,
        selected_frame: usize,
        macroblock_index: usize,
    ) {
        let frame = &frames[selected_frame];
        let info = &frame.stats.macroblock_info[macroblock_index];

        self.render_macroblock_result(&frame.frame, macroblock_index);

        match &info.kind {
            MacroblockInfoKind::Intra => {
                self.render_previous_reference(frames, selected_frame, macroblock_index);
            }
            MacroblockInfoKind::Moved {
                is_forward: true,
                before_diff,
                ..
            } => {
                self.render_previous_reference(frames, selected_frame, macroblock_index);
                self.draw_macroblock(&self.canvas_history_previous_before_diff, &before_diff);
            }
            MacroblockInfoKind::Moved {
                is_forward: false,
                before_diff,
                ..
            } => {
                self.render_next_reference(frames, selected_frame, macroblock_index);
                self.draw_macroblock(&self.canvas_history_next_before_diff, &before_diff);
            }
            MacroblockInfoKind::Interpolated {
                forward,
                backward,
                interpolated,
                ..
            } => {
                self.render_previous_reference(frames, selected_frame, macroblock_index);
                self.render_next_reference(frames, selected_frame, macroblock_index);
                self.draw_macroblock(&self.canvas_history_previous_before_diff, &forward);
                self.draw_macroblock(&self.canvas_history_next_before_diff, &backward);
                self.draw_macroblock(&self.canvas_history_interpolated, &interpolated);
            }
            MacroblockInfoKind::Skipped => {}
        }
    }

    fn render_macroblock_result(&self, frame: &VideoFrame, macroblock_address: usize) {
        self.draw_macroblock_from_frame(frame, macroblock_address, &self.canvas_history_result);
    }

    fn draw_macroblock_from_frame(
        &self,
        frame: &VideoFrame,
        macroblock_address: usize,
        target: &ElRef<HtmlCanvasElement>,
    ) {
        let mut y1 = [0; 64];
        let mut y2 = [0; 64];
        let mut y3 = [0; 64];
        let mut y4 = [0; 64];
        let mut cb = [0; 64];
        let mut cr = [0; 64];

        let macroblock_width = (self.width as usize + 15) / 16;
        let y = (macroblock_address / macroblock_width) * 16;
        let x = (macroblock_address % macroblock_width) * 16;

        self.get_block(x, y, &mut y1, &frame.y, macroblock_width);
        self.get_block(x + 8, y, &mut y2, &frame.y, macroblock_width);
        self.get_block(x, y + 8, &mut y3, &frame.y, macroblock_width);
        self.get_block(x + 8, y + 8, &mut y4, &frame.y, macroblock_width);

        let chroma_y = y / 2;
        let chroma_x = x / 2;

        self.get_block(chroma_x, chroma_y, &mut cr, &frame.cr, macroblock_width / 2);
        self.get_block(chroma_x, chroma_y, &mut cb, &frame.cb, macroblock_width / 2);

        let macroblock_content = MacroblockContent {
            y1,
            y2,
            y3,
            y4,
            cb,
            cr,
        };

        self.draw_macroblock(target, &macroblock_content);
    }

    fn draw_macroblock(&self, target: &ElRef<HtmlCanvasElement>, macroblock: &MacroblockContent) {
        let ys = [
            &macroblock.y1,
            &macroblock.y2,
            &macroblock.y3,
            &macroblock.y4,
        ];

        let mut image_data = Vec::with_capacity(16 * 16 * 4);
        for row in 0..16 {
            for col in 0..16 {
                let y_buffer_index = 2 * (row / 8) + (col / 8);
                let y_index = (row % 8) * 8 + col % 8;
                let chroma_index = (row / 2) * 8 + (col / 2);

                let y = ys[y_buffer_index][y_index];
                let cb = macroblock.cb[chroma_index];
                let cr = macroblock.cr[chroma_index];

                let ycbr = crate::image::pixel::YCbCr { y, cb, cr };
                let RGB { r, g, b } = ycbr.to_rgb();
                image_data.extend([r, g, b, 255]);
            }
        }

        let image_data =
            ImageData::new_with_u8_clamped_array_and_sh(Clamped(&mut image_data), 16, 16).unwrap();
        let context = canvas_context_2d(&target.get().unwrap());
        context.set_image_smoothing_enabled(false);
        draw_scaled_image_with_image_data_with_w_h_and_scale(
            &target,
            &image_data,
            16,
            16,
            8.0,
            8.0,
        );
    }

    fn render_previous_reference(
        &self,
        frames: &[DecodedFrame],
        selected_frame: usize,
        macroblock_index: usize,
    ) {
        let target = &self.canvas_history_previous_reference;

        if selected_frame == 0 {
            let context = canvas_context_2d(&target.get().unwrap());
            context.clear_rect(0.0, 0.0, self.width.into(), self.height.into());
            return;
        }

        let previous_reference_frame = frames[0..selected_frame]
            .iter()
            .rev()
            .find(|frame| {
                matches!(
                    frame.stats.picture_type,
                    constants::PICTURE_TYPE_INTRA | constants::PICTURE_TYPE_PREDICTIVE
                )
            })
            .unwrap();

        self.draw_macroblock_from_frame(&previous_reference_frame.frame, macroblock_index, target);
    }

    fn render_next_reference(
        &self,
        frames: &[DecodedFrame],
        selected_frame: usize,
        macroblock_index: usize,
    ) {
        let next_reference_frame = frames[selected_frame..frames.len()]
            .iter()
            .find(|frame| {
                matches!(
                    frame.stats.picture_type,
                    constants::PICTURE_TYPE_INTRA | constants::PICTURE_TYPE_PREDICTIVE
                )
            })
            .unwrap();

        self.draw_macroblock_from_frame(
            &next_reference_frame.frame,
            macroblock_index,
            &self.canvas_history_next_reference,
        );
    }
}

enum ChannelType {
    Y,
    Cb,
    Cr,
}
