use crate::{quant, dct, BLOCK_SIZE};


pub struct Block(pub [[u8; 8]; 8]);

pub struct BlockMatrix {
    pub blocks: Vec<Block>,
    pub width: usize,
    pub height: usize
}

pub fn split_to_block_matrix(data: &Vec<u8>) -> BlockMatrix {
    let block_count = data.len() / (8 * BLOCK_SIZE as usize);
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
            result[y][x] = data[(v * 8 + y) * BLOCK_SIZE as usize + (u * 8) + x];
        }
    }

    Block(result)
}