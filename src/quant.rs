pub fn apply_quantization(data: &mut [[i16; 8]; 8], quantization: &[[u8; 8]; 8]) {
    for y in 0..8 {
        for x in 0..8 {
            data[y][x] /= quantization[y][x] as i16;
        }
    }
}

pub fn undo_quantization(data: &mut [[i16; 8]; 8], quantization: &[[u8; 8]; 8]) {
    for y in 0..8 {
        for x in 0..8 {
            data[y][x] *= quantization[y][x] as i16;
        }
    }
}

pub fn scale_quantization_table(quantization_table: &[[u8; 8]; 8], quality: u8) -> [[u8; 8]; 8] {
    let scaling_factor = 100 - quality;

    let mut scaled_quantization_table: [[u8; 8]; 8] = [[0; 8]; 8];

    for y in 0..8 {
        for x in 0..8 {
            scaled_quantization_table[y][x] = std::cmp::max(
                ((quantization_table[y][x] as u32 * scaling_factor as u32) / 100) as u8,
                1,
            );
        }
    }
    scaled_quantization_table
}

pub const LUMINANCE_QUANTIZATION_TABLE: [[u8; 8]; 8] = [
    [16, 11, 10, 16, 24, 40, 51, 61],
    [12, 12, 14, 19, 26, 58, 60, 55],
    [14, 13, 16, 24, 40, 57, 69, 56],
    [14, 17, 22, 29, 51, 87, 80, 62],
    [18, 22, 37, 56, 68, 109, 103, 77],
    [24, 35, 55, 64, 81, 104, 113, 92],
    [49, 64, 78, 87, 103, 121, 120, 101],
    [72, 92, 95, 98, 112, 100, 103, 99],
];

pub const CHROMINANCE_QUANTIZATION_TABLE: [[u8; 8]; 8] = [
    [17, 18, 24, 47, 99, 99, 99, 99],
    [18, 21, 26, 66, 99, 99, 99, 99],
    [24, 26, 56, 99, 99, 99, 99, 99],
    [47, 66, 99, 99, 99, 99, 99, 99],
    [99, 99, 99, 99, 99, 99, 99, 99],
    [99, 99, 99, 99, 99, 99, 99, 99],
    [99, 99, 99, 99, 99, 99, 99, 99],
    [99, 99, 99, 99, 99, 99, 99, 99],
];
