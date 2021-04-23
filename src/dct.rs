use std::f32::consts::{FRAC_1_SQRT_2, PI};
use crate::log;

pub fn spatial_to_freq(block: &[[u8; 8]; 8]) -> [[u8; 8]; 8] {
    let mut result = [[0 as u8; 8]; 8];

    for x in 0..8 {
        for y in 0..8 {
            result[y][x] = G(y, x, block).abs().clamp(0.0, 255.0) as u8;
        }
    }

    result
}

fn G(u: usize, v: usize, block: &[[u8; 8]; 8]) -> f32 {
    let (u, v) = (u as f32, v as f32);
    let au = if u == 0.0 { FRAC_1_SQRT_2 } else { 1.0 };
    let av = if v == 0.0 { FRAC_1_SQRT_2 } else { 1.0 };

    let mut sum = 0.0;
    for x in 0..8 {
        for y in 0..8 {
            let g = block[y][x] as f32 - 128.0;
            let cosx = (((2 * x + 1) as f32 * u * PI) / 16.0).cos();
            let cosy = (((2 * y + 1) as f32 * v * PI) / 16.0).cos();

            sum += g * cosx * cosy;
        }
    }

    0.25 * au * av * sum
}
