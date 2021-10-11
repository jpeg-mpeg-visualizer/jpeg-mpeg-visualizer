use crate::{dct, quant, BLOCK_SIZE};

pub struct Block(pub [[i16; 8]; 8]);

pub struct BlockMatrix {
    pub blocks: Vec<Block>,
    pub width: usize,
    pub height: usize,
}

pub fn split_to_block_matrix(data: &[u8]) -> BlockMatrix {
    let block_count = data.len() / (8 * BLOCK_SIZE as usize);
    let mut blocks: Vec<Block> = Vec::with_capacity(block_count * block_count);

    for v in 0..block_count {
        for u in 0..block_count {
            blocks.push(get_block(u, v, &data));
        }
    }
    BlockMatrix {
        blocks,
        height: block_count,
        width: block_count,
    }
}

impl BlockMatrix {
    pub fn apply_quantization(&self, quantization: &[[u8; 8]; 8]) -> BlockMatrix {
        let mut quantized_blocks: Vec<Block> = Vec::with_capacity(self.width * self.height);
        for v in 0..self.height {
            for u in 0..self.width {
                let mut spatial = dct::spatial_to_freq(&self.blocks[u + v * self.width].0);
                quant::apply_quantization(&mut spatial, quantization);
                quantized_blocks.push(Block(spatial));
            }
        }
        BlockMatrix {
            blocks: quantized_blocks,
            width: self.width,
            height: self.height,
        }
    }

    pub fn flatten(&self) -> Vec<u8> {
        let mut result: Vec<u8> = vec![0; self.width * self.height * 8 * 8];
        for y in 0..self.height {
            for x in 0..self.width {
                for iy in 0..8 {
                    for ix in 0..8 {
                        result[((y * 8) + iy) * self.width * 8 + x * 8 + ix] =
                            self.blocks[y * self.width + x].0[iy][ix] as u8;
                    }
                }
            }
        }
        result
    }

    pub fn undo_quantization(&self, quantization: &[[u8; 8]; 8]) -> BlockMatrix {
        let mut result: Vec<Block> = Vec::with_capacity(self.width * self.height);
        for v in 0..self.height {
            for u in 0..self.width {
                let mut freq = self.blocks[u + v * self.width].0;
                quant::undo_quantization(&mut freq, quantization);
                let spatial = dct::freq_to_spatial(&freq);
                result.push(Block(spatial));
            }
        }
        BlockMatrix {
            blocks: result,
            width: self.width,
            height: self.height,
        }
    }
}

fn get_block(u: usize, v: usize, data: &[u8]) -> Block {
    let mut result = [[0_i16; 8]; 8];

    for y in 0..8 {
        for x in 0..8 {
            result[y][x] = data[(v * 8 + y) * BLOCK_SIZE as usize + (u * 8) + x] as i16;
        }
    }

    Block(result)
}
