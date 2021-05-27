use std::f32::consts::{FRAC_1_SQRT_2, PI};

#[allow(non_snake_case)]
pub fn spatial_to_freq(block: &[[i16; 8]; 8]) -> [[i16; 8]; 8] {
    let mut result = [[0 as i16; 8]; 8];

    for x in 0..8 {
        for y in 0..8 {
            result[y][x] = G(x, y, block) as i16;
        }
    }

    result
}

#[allow(non_snake_case)]
fn G(u: usize, v: usize, block: &[[i16; 8]; 8]) -> f32 {
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

pub fn freq_to_spatial(block: &[[i16; 8]; 8]) -> [[i16; 8]; 8] {
    let mut result = [[0 as i16; 8]; 8];

    for x in 0..8 {
        for y in 0..8 {
            result[y][x] = (f(y, x, block) + 128.0) as i16;
        }
    }

    result
}

fn f(x: usize, y: usize, block: &[[i16; 8]; 8]) -> f32 {
    let mut sum = 0.0;
    for u in 0..8 {
        for v in 0..8 {
            let au = if u == 0 { FRAC_1_SQRT_2 } else { 1.0 };
            let av = if v == 0 { FRAC_1_SQRT_2 } else { 1.0 };

            let ff = block[u][v] as f32;
            let (u, v) = (u as f32, v as f32);
            let cosx = (((2 * x + 1) as f32 * u * PI) / 16.0).cos();
            let cosy = (((2 * y + 1) as f32 * v * PI) / 16.0).cos();
            sum += ff * au * av * cosx * cosy
        }
    }
    
    0.25 * sum
}

mod test {
    use super::freq_to_spatial;

    #[test]
    pub fn test_freq_to_spatial() {
        let freq_block: [[i16; 8]; 8] = [
            [-416, -33, -60, 32, 48, -40, 0, 0],
            [0, -24, -56, 19, 26, 0, 0, 0],
            [-42, 13, 80, -24, -40, 0, 0, 0],
            [-42, 17, 44, -29, 0, 0, 0, 0],
            [18, 0, 0, 0, 0, 0, 0, 0],
            [0, 0, 0, 0, 0, 0, 0, 0],
            [0, 0, 0, 0, 0, 0, 0, 0],
            [0, 0, 0, 0, 0, 0, 0, 0]
        ];
            
        dbg!(freq_to_spatial(&freq_block));
    }
}