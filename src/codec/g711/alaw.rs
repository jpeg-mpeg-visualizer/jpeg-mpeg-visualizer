use crate::codec::g711::{SoundDecoder, SoundEncoder};
use crate::section::g711_visualization::model::PlayerState;

pub struct ALawCodec {}

const SEG_SHIFT: u8 = 4;
const SEG_MASK: u8 = 0x70;
const QUANT_MASK: u8 = 0xf;
const SIGN_BIT: u8 = 0x80;

const SEG_AEND: [i16; 8] = [0x1F, 0x3F, 0x7F, 0xFF, 0x1FF, 0x3FF, 0x7FF, 0xFFF];

impl SoundEncoder for ALawCodec {
    fn encode_frame(&self, input_pcm: i16) -> u8 {
        let mut temp_pcm = input_pcm >> 3;
        let mut mask: i16;
        if temp_pcm >= 0 {
            mask =0xD5;
        } else {
            mask = 0x55;
            temp_pcm = -temp_pcm -1;
        }
        let seg = SEG_AEND.iter()
            .position(|&value| temp_pcm <= value)
            .unwrap_or(SEG_AEND.len()) as i16;

        if seg >= 8 {
            (0x7F ^ mask) as u8
        } else {
            let mut aval = seg << SEG_SHIFT;
            if seg < 2 {
                aval |= (temp_pcm >> 1) & QUANT_MASK as i16;
            } else {
                aval |= (temp_pcm >> seg) & QUANT_MASK as i16;
            }
            (aval ^ mask) as u8
        }
    }
}

impl SoundDecoder for ALawCodec {
    fn decode_frame(&self, input_8bit: u8) -> i16 {
        let val = input_8bit ^ 0x55;

        let mut temp = (val as i16 & QUANT_MASK as i16) << 4;
        let seg = (val & SEG_MASK) >> SEG_SHIFT;
        match seg {
            0 => {
                temp += 8;
            }
            1 => {
                temp += 0x108;
            }
            _ => {
                temp += 0x108;
                temp <<= seg - 1;
            }
        }
        if val & SIGN_BIT == SIGN_BIT {
            temp
        } else {
            -temp
        }
    }
}