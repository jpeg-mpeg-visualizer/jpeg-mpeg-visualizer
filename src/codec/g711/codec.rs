pub trait SoundDecoder where {
    fn decode_frame(&self, input_8bit: u8) -> i16;

    fn decode_frames(&self, input_8bits: &Vec<u8>) -> Vec<i16> {
        input_8bits
            .iter()
            .map(|frame| Self::decode_frame(self,*frame))
            .collect::<Vec<i16>>()
    }
}

pub trait SoundEncoder {
    fn encode_frame(&self, input_pcm: i16) -> u8;

    fn encode_frames(&self, input_pcm: &Vec<i16>) -> Vec<u8> {
        input_pcm
            .iter()
            .map(|frame| Self::encode_frame(self,*frame))
            .collect::<Vec<u8>>()
    }
}

pub trait SoundCodec: SoundEncoder + SoundDecoder {}