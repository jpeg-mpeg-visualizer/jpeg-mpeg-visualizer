use crate::{quant, dct};

pub struct Block(pub [[u8; 8]; 8]);

pub struct BlockMatrix {
    pub(crate) blocks: Vec<Block>,
    pub(crate) width: usize,
    pub(crate) height: usize
}

impl BlockMatrix {
    pub fn apply_quantization(&self, quantization: &[[u8; 8]; 8]) -> Vec<[[u8; 8]; 8]>{
        let mut quantized_blocks: Vec<[[u8; 8]; 8]> = Vec::with_capacity(self.width*self.height);
        for v in 0..self.height {
            for u in 0..self.width {
                let mut spatial = dct::spatial_to_freq(&self.blocks[u+v*self.width].0);
                quant::apply_quantization(&mut spatial, quantization);
                quantized_blocks.push(spatial);
            }
        }
        quantized_blocks
    }
}

fn get_block(u: usize, v: usize, data: &Vec<u8>) -> Block {
    let mut result = [[0 as u8; 8]; 8];

    for y in 0..8 {
        for x in 0..8 {
            result[y][x] = data[(v * 8 + y) * 500 + (u * 8) + x];
        }
    }

    Block(result)
}

pub fn split_to_block_matrix(data: &Vec<u8>) -> BlockMatrix {
    let block_count = data.len() / (8 * 500);
    let mut blocks: Vec<Block> = Vec::with_capacity(block_count*block_count);

    for v in 0..block_count {
        for u in 0..block_count {
            blocks.push(get_block(u, v, &data));
        }
    }
    BlockMatrix{
        blocks,
        height: block_count,
        width: block_count
    }
}

pub fn apply_quantization(data: &mut [[u8; 8]; 8], quantization: &[[u8; 8]; 8]) {
    for y in 0..8 {
        for x in 0..8 {
            data[y][x] /= quantization[y][x];
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
                1
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
