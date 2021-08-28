use std::f32::consts::{FRAC_1_SQRT_2, PI};

#[allow(non_snake_case)]
pub fn spatial_to_freq(block: &[[i16; 8]; 8]) -> [[i16; 8]; 8] {
    let mut result = [[0 as i16; 8]; 8];

    for x in 0..8 {
        for y in 0..8 {
            result[y][x] = G(x, y, block).round() as i16;
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
            // we clamp the result, because it could land outside [0, 255] range after the dequantization step
            // it would be flipped after the conversion to u8, this caused the "burned in" pixels
            result[y][x] = (f(y, x, block) + 128.0).round().clamp(0.0, 255.0) as i16;
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
    use std::convert::TryFrom;

    use super::{freq_to_spatial, spatial_to_freq};

    #[test]
    pub fn test_spatial_to_freq() {
        // example from https://en.wikipedia.org/wiki/JPEG#Discrete_cosine_transform
        let spatial_block: [[i16; 8]; 8] = [
            [52, 55, 61, 66, 70, 61, 64, 73],
            [63, 59, 55, 90, 109, 85, 69, 72],
            [62, 59, 68, 113, 144, 104, 66, 73],
            [63, 58, 71, 122, 154, 106, 70, 69],
            [67, 61, 68, 104, 126, 88, 68, 70],
            [79, 65, 60, 70, 77, 68, 58, 75],
            [85, 71, 64, 59, 55, 61, 65, 83],
            [87, 79, 69, 68, 65, 76, 78, 94],
        ];

        let expected_freq_block = <[[i16; 8]; 8]>::try_from(
            [
                [-415.38, -30.19, -61.20, 27.24, 56.12, -20.10, -2.39, 0.46],
                [4.47, -21.86, -60.76, 10.25, 13.15, -7.09, -8.54, 4.88],
                [-46.83, 7.37, 77.13, -24.56, -28.91, 9.93, 5.42, -5.65],
                [-48.53, 12.07, 34.10, -14.76, -10.24, 6.30, 1.83, 1.95],
                [12.12, -6.55, -13.20, -3.95, -1.87, 1.75, -2.79, 3.14],
                [-7.73, 2.91, 2.38, -5.94, -2.38, 0.94, 4.30, 1.85],
                [-1.03, 0.18, 0.42, -2.42, -0.88, -3.02, 4.12, -0.66],
                [-0.17, 0.14, -1.07, -4.19, -1.17, -0.10, 0.50, 1.68],
            ]
            .iter()
            .map(|row| {
                <[i16; 8]>::try_from(
                    row.iter()
                        .map(|x| (*x as f32).round() as i16)
                        .collect::<Vec<i16>>(),
                )
                .unwrap()
            })
            .collect::<Vec<[i16; 8]>>(),
        )
        .unwrap();

        assert_eq!(spatial_to_freq(&spatial_block), expected_freq_block);
    }
    
    #[test]
    pub fn test_freq_to_spatial() {
        // example from https://en.wikipedia.org/wiki/JPEG#Decoding
        let freq_block: [[i16; 8]; 8] = [
            [-416, -33, -60, 32, 48, -40, 0, 0],
            [0, -24, -56, 19, 26, 0, 0, 0],
            [-42, 13, 80, -24, -40, 0, 0, 0],
            [-42, 17, 44, -29, 0, 0, 0, 0],
            [18, 0, 0, 0, 0, 0, 0, 0],
            [0, 0, 0, 0, 0, 0, 0, 0],
            [0, 0, 0, 0, 0, 0, 0, 0],
            [0, 0, 0, 0, 0, 0, 0, 0],
        ];

        let expected_spatial_block: [[i16; 8]; 8] = [
            [62, 65, 57, 60, 72, 63, 60, 82],
            [57, 55, 56, 82, 108, 87, 62, 71],
            [58, 50, 60, 111, 148, 114, 67, 65],
            [65, 55, 66, 120, 155, 114, 68, 70],
            [70, 63, 67, 101, 122, 88, 60, 78],
            [71, 71, 64, 70, 80, 62, 56, 81],
            [75, 82, 67, 54, 63, 65, 66, 83],
            [81, 94, 75, 54, 68, 81, 81, 87],
        ];

        assert_eq!(freq_to_spatial(&freq_block), expected_spatial_block);
    }
}
