use crate::codec::g711::{SoundDecoder, SoundEncoder};
use super::codec::SoundCodec;

const QUANT_MASK: u8 = 0xf;
const BIAS: u8 = 0x84;
const SEG_MASK: u8 = 0x70;
const SEG_SHIFT: u8 = 4;
const SIGN_BIT: u8 = 0x80;
const CLIP: i16 = 8159;

const SEG_UEND: [i16; 8] = [0x3F, 0x7F, 0xFF, 0x1FF, 0x3FF, 0x7FF, 0xFFF, 0x1FFF];

pub struct ULawCodec {}

impl SoundEncoder for ULawCodec {
    fn encode_frame(&self, input_pcm: i16) -> u8 {
        let mut temp_pcm = input_pcm >> 2;
        let mut mask: i16;
        if temp_pcm < 0 {
            temp_pcm = - temp_pcm;
            mask = 0x7F;
        } else {
            mask = 0xFF
        }

        temp_pcm = std::cmp::min(temp_pcm, CLIP);
        temp_pcm += (BIAS >> 2) as i16;

        let seg = SEG_UEND.iter()
            .position(|&value|  temp_pcm <= value)
            .unwrap_or(SEG_UEND.len()) as i16;

        let result = if seg >= 8 {
            0x7F ^ mask
        } else {
            ((seg << 4) | ((temp_pcm >> (seg + 1)) & 0xF)) ^ mask
        };
        result as u8
    }
}

impl SoundDecoder for ULawCodec {
    fn decode_frame(&self, input_8bit: u8) -> i16 {
        let not_complemented = !input_8bit;

        let mut decoded: i16 = (((not_complemented as i16 & QUANT_MASK as i16) << 3) + BIAS as i16) as i16;
        decoded <<= (not_complemented as i16 & SEG_MASK as i16) >> SEG_SHIFT;

        if (not_complemented & SIGN_BIT) == SIGN_BIT {
            BIAS as i16 - decoded
        } else {
            decoded - BIAS as i16
        }
    }
}

#[cfg(test)]
mod test {
    use crate::codec::g711::{SoundDecoder, SoundEncoder};
    use super::ULawCodec;

    #[test]
    pub fn test_decode_frame() {
        let frames_u8: Vec<u8> = vec![
            0xFF, 0x7F, 0x47, 0xD7
        ];

        let frames_pcm: Vec<i16> = vec![
            0, 0, -1436, 652
        ];

        let decoder = ULawCodec {};
        assert_eq!(decoder.decode_frames(&frames_u8), frames_pcm);
    }

    #[test]
    pub fn test_encode_frame() {
        let frames_pcm: Vec<i16> = vec![
             0, -1, 21, 8111
        ];

        let frames_u8: Vec<u8> = vec![
            0xFF, 0x7E, 0xFC, 0x9F
        ];

        let decoder = ULawCodec {};
        assert_eq!(decoder.encode_frames(&frames_pcm), frames_u8);
    }
}
