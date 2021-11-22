use std::cmp::Ordering;

use rustfft::{algorithm::Radix4, num_complex::Complex, num_traits::Zero, Fft, FftDirection};
use seed::*;
use wasm_bindgen::Clamped;
use web_sys::{HtmlCanvasElement, ImageData};

pub struct Spectrogram {
    pub image_data: Vec<u8>,
    pub windows_num: usize,
    pub bins_num: usize,
}

impl Spectrogram {
    pub fn new(pcm: &[i16]) -> Spectrogram {
        const WINDOW_SIZE: usize = 2048;
        const HOP_SIZE: usize = 1024;
        const MAX_DB_DROP: f32 = 80.0;

        let buffer = pcm
            .iter()
            .map(|x| Complex {
                re: *x as f32,
                im: 0.0,
            })
            .collect::<Vec<_>>();

        let windows_num = 1 + (buffer.len() - WINDOW_SIZE) / HOP_SIZE;
        let bins_num = WINDOW_SIZE / 2 + 1;
        let windows = buffer.windows(WINDOW_SIZE).step_by(HOP_SIZE);

        let fft = Radix4::<f32>::new(WINDOW_SIZE, FftDirection::Forward);
        let mut result: Vec<Vec<f32>> = Vec::with_capacity(windows_num);
        let mut scratch = vec![Complex::zero(); fft.get_inplace_scratch_len()];

        for chunk in windows {
            let mut spectrum = chunk.to_owned();
            fft.process_with_scratch(&mut spectrum, &mut scratch);
            let spectrum = spectrum[0..bins_num]
                .iter()
                .map(|x| power_to_db(x.norm_sqr()))
                .collect::<Vec<_>>();
            result.push(spectrum);
        }

        for row in result.iter_mut() {
            row.reverse()
        }
        let result = transpose(result);

        let flattened_result: Vec<f32> = result.into_iter().flatten().collect();
        let max_value = get_extrema(&flattened_result, true);
        let min_value = f32::max(
            get_extrema(&flattened_result, false),
            max_value - MAX_DB_DROP,
        );

        let color_map = colorous::INFERNO;
        let mut image_data = Vec::with_capacity(windows_num * bins_num * 4);

        for bin in flattened_result.iter() {
            let normalized_bin =
                (bin.clamp(min_value, max_value) - min_value) / (max_value - min_value);
            let color = color_map.eval_continuous(normalized_bin.into());
            image_data.extend(color.as_array());
            image_data.push(255);
        }

        Spectrogram {
            image_data,
            windows_num,
            bins_num,
        }
    }

    pub fn draw_spectrogram(&self, canvas: HtmlCanvasElement) {
        let image_data = ImageData::new_with_u8_clamped_array_and_sh(
            Clamped(&self.image_data),
            self.windows_num as u32,
            self.bins_num as u32,
        )
        .expect("Encountered error while creating spectrogram's ImageData");

        canvas.set_width(self.windows_num as u32);
        canvas.set_height(self.bins_num as u32);

        let context = canvas_context_2d(&canvas);
        context.put_image_data(&image_data, 0.0, 0.0).unwrap();
    }
}

fn transpose<T>(v: Vec<Vec<T>>) -> Vec<Vec<T>> {
    let len = v[0].len();
    let mut iters: Vec<_> = v.into_iter().map(|n| n.into_iter()).collect();
    (0..len)
        .map(|_| {
            iters
                .iter_mut()
                .map(|n| n.next().unwrap())
                .collect::<Vec<T>>()
        })
        .collect()
}

fn power_to_db(power: f32) -> f32 {
    10.0 * power.log10()
}

fn get_extrema(values: &[f32], max: bool) -> f32 {
    let compare_function = |a: &&f32, b: &&f32| {
        let ordering = a.partial_cmp(b).unwrap_or(Ordering::Equal);
        if max {
            ordering
        } else {
            ordering.reverse()
        }
    };

    *values
        .iter()
        .filter(|x| x.is_normal())
        .max_by(compare_function)
        .expect("Empty `values` slice")
}
