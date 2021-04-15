pub fn apply_quantization(data: &mut Vec<Vec<u8>>, quality: u8, luminance: bool) {
    assert!(1 <= quality && quality <= 100);

    let table = if luminance {
        LUMINANCE_QUANTIZATION_TABLE
    } else {
        CHROMINANCE_QUANTIZATION_TABLE
    };

    let scaling_factor = 100 - quality;

    for y in 0..8 {
        for x in 0..8 {
            let quantization_value = std::cmp::max(
                ((table[y][x] as u32 * scaling_factor as u32) / 100) as u8,
                1,
            );
            data[y][x] /= quantization_value;
        }
    }
}

const LUMINANCE_QUANTIZATION_TABLE: [[u8; 8]; 8] = [
    [16, 11, 10, 16, 24, 40, 51, 61],
    [12, 12, 14, 19, 26, 58, 60, 55],
    [14, 13, 16, 24, 40, 57, 69, 56],
    [14, 17, 22, 29, 51, 87, 80, 62],
    [18, 22, 37, 56, 68, 109, 103, 77],
    [24, 35, 55, 64, 81, 104, 113, 92],
    [49, 64, 78, 87, 103, 121, 120, 101],
    [72, 92, 95, 98, 112, 100, 103, 99],
];

const CHROMINANCE_QUANTIZATION_TABLE: [[u8; 8]; 8] = [
    [17, 18, 24, 47, 99, 99, 99, 99],
    [18, 21, 26, 66, 99, 99, 99, 99],
    [24, 26, 56, 99, 99, 99, 99, 99],
    [47, 66, 99, 99, 99, 99, 99, 99],
    [99, 99, 99, 99, 99, 99, 99, 99],
    [99, 99, 99, 99, 99, 99, 99, 99],
    [99, 99, 99, 99, 99, 99, 99, 99],
    [99, 99, 99, 99, 99, 99, 99, 99],
];
