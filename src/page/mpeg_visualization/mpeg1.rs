use bitvec::prelude::*;
use seed::*;

use crate::page::mpeg_visualization::mpeg1::constants::{EXTENSION_START_CODE, SLICE_LAST_START_CODE, USER_DATA_START_CODE};

pub struct MPEG1 {
    pointer: usize,
    buffer: BitVec<Msb0, u8>,
    has_sequence_header: bool,
    
    width: u16,
    height: u16,
    mb_width: u16,
    mb_row: usize,
    mb_col: usize,

    picture_type: u8,
    
    current_y: Vec<u8>,
    current_cr: Vec<u8>,
    current_cb: Vec<u8>,
    forward_y: Vec<u8>,
    forward_cr: Vec<u8>,
    forward_cb: Vec<u8>,
    
    intra_quant_matrix: [u8; 64],
    non_intra_quant_matrix: [u8; 64],
    
    slice_beginning: bool,
    macroblock_address: usize,
}

impl MPEG1 {
    pub fn from_bytes(bytes: Vec<u8>) -> MPEG1 {
        MPEG1 {
            pointer: 0,
            buffer: BitVec::from_vec(bytes),
            has_sequence_header: false,
            width: 0,
            height: 0,
            mb_width: 0,
            mb_row: 0,
            mb_col: 0,
            picture_type: 0,
            current_y: Vec::new(),
            current_cr: Vec::new(),
            current_cb: Vec::new(),
            forward_y: Vec::new(),
            forward_cr: Vec::new(),
            forward_cb: Vec::new(),
            intra_quant_matrix: constants::DEFAULT_INTRA_QUANT_MATRIX,
            non_intra_quant_matrix: constants::DEFAULT_NON_INTRA_QUANT_MATRIX,
            slice_beginning: false,
            macroblock_address: 0,
        }
    }
    
    pub fn decode(&mut self) {
        if !self.has_sequence_header {
            self.find_start_code(constants::SEQUENCE_HEADER_CODE);
            self.decode_sequence_header();
        }

        self.find_start_code(constants::PICTURE_START_CODE);
        self.decode_picture();
    }
    
    fn get_next_start_code(&mut self) -> u32 {
        // byte align the pointer
        self.pointer = ((self.pointer + 7) / 8) * 8;
        while self.buffer[self.pointer..self.pointer+24].load_be::<u32>() != 0x00_00_01 {
            self.pointer += 8;
        } 
        self.buffer[self.pointer..self.pointer+32].load_be::<u32>()
    }
    
    fn find_start_code(&mut self, code: u32) {
        while self.buffer[self.pointer..self.pointer+32].load_be::<u32>() != code {
            self.pointer += 8;
        }
    }
    
    fn next_bytes_are_start_code(&mut self) -> bool {
        let aligned_pointer = ((self.pointer + 7) / 8) * 8;
        self.buffer[aligned_pointer..aligned_pointer+24].load_be::<u32>() == 0x00_00_01
    }
    
    fn init_buffers(&mut self, width: u16, height: u16) {
        self.mb_width = (width + 15) / 16;
        let mb_height = (height + 15) / 16;
        let mb_size = self.mb_width * mb_height;

        let coded_width = self.mb_width / 16;
        let coded_height = mb_height / 16;
        let coded_size = coded_width * coded_height;

        self.current_y = vec![0; coded_size as usize];
        self.current_cb = vec![0; coded_size as usize / 2];
        self.current_cr = vec![0; coded_size as usize / 2];

        self.forward_y = vec![0; coded_size as usize];
        self.forward_cb = vec![0; coded_size as usize / 2];
        self.forward_cr = vec![0; coded_size as usize / 2];
    }
    
    fn decode_sequence_header(&mut self) {
        // Skip over sequence header start code
        self.pointer += 32; 

        let width = self.buffer[self.pointer..self.pointer+12].load_be::<u16>(); 
        let height = self.buffer[self.pointer+12..self.pointer+12+12].load_be::<u16>(); 
        
        // Skip over 39 bits of data:
        // Pel aspect ratio - 4 bits
        // Picture rate - 4 bits
        // Bit rate - 18 bits
        // Marker bit - 1 bit
        // Vbv buffer size - 10 bit
        // Constrained parameters flag - 1 bit
        self.pointer += 12 + 12 + 4 + 4 + 18 + 1 + 10 + 1;
        
        // TODO init buffers or smth
        if (width, height) != (self.width, self.height) {
            self.width = width;
            self.height = height;

            self.init_buffers(width, height);
        }

        let load_intra_quantizer_matrix = self.buffer[self.pointer..self.pointer+1].load::<u8>();
        self.pointer += 1;

        if load_intra_quantizer_matrix == 1 {
            for i in 0..64 {
                self.intra_quant_matrix[constants::ZIG_ZAG[i]] = self.buffer[self.pointer..self.pointer+8].load::<u8>(); 
                self.pointer += 8;
            } 
        }

        let load_non_intra_quantizer_matrix = self.buffer[self.pointer..self.pointer+1].load::<u8>();
        self.pointer += 1;

        if load_non_intra_quantizer_matrix == 1 {
            for i in 0..64 {
                self.non_intra_quant_matrix[constants::ZIG_ZAG[i]] = self.buffer[self.pointer..self.pointer+8].load::<u8>(); 
                self.pointer += 8;
            } 
        }
        
        self.has_sequence_header = true;
        
        log(width);
        log(height);
        log(self.pointer);
    }
    
    fn decode_picture(&mut self) {
        // Skip over picture start code
        self.pointer += 32; 
        
        // Skip over temporal reference
        self.pointer += 10;
        
        log(self.buffer[self.pointer..self.pointer+3].to_string());
        self.picture_type = self.buffer[self.pointer..self.pointer+3].load::<u8>();
        
        // Skip over VBV buffer delay
        self.pointer += 3 + 16;
        
        // Assert that the picture is 
        assert!(self.picture_type == constants::PICTURE_TYPE_INTRA || self.picture_type == constants::PICTURE_TYPE_PREDICTIVE);

        if self.picture_type == constants::PICTURE_TYPE_PREDICTIVE {
            let full_pel_forward = self.buffer[self.pointer..self.pointer+1].load::<u8>();
            let forward_f_code = self.buffer[self.pointer+1..self.pointer+1+3].load::<u8>();
            self.pointer += 4;
        }
        
        let mut start_code: u32 = self.get_next_start_code();
        while start_code == constants::USER_DATA_START_CODE || start_code == constants::EXTENSION_START_CODE {
            self.pointer += 32;
            start_code = self.get_next_start_code();
        }
        
        while let constants::SLICE_FIRST_START_CODE..=constants::SLICE_LAST_START_CODE = start_code {
            self.decode_slice((start_code & 0x00_00_00_FF) as u16);
            start_code = self.get_next_start_code()
        }
        
        // TODO I guess here we will return the decoded frame 
    }
    
    fn decode_slice(&mut self, slice: u16) {
        self.slice_beginning = true;

        // Skip over slice start code
        self.pointer += 32;

        self.macroblock_address = ((slice - 1) * self.mb_width - 1) as usize;
        
        // TODO Reset motion vectors and DC predictors
        // 

        let quantizer_scale = self.buffer[self.pointer..self.pointer+5].load::<u8>();
        self.pointer += 1;


        // Skip over extra information
        while self.buffer[self.pointer] {
            self.pointer += 1 + 8; 
        }
        self.pointer += 1;
        
        // There must be at least one macroblock
        loop {
            self.decode_macroblock();
            if self.next_bytes_are_start_code() { break };
        }
    }
    
    fn decode_macroblock(&mut self) {
        let mut increment = 0;
        let mut t = self.read_huffman(&constants::MACROBLOCK_ADDRESS_INCREMENT);

        // Skip macroblock_stuffing
        while t == 34 {
            t = self.read_huffman(&constants::MACROBLOCK_ADDRESS_INCREMENT);
        }

        // Handle macroblock_escape
        while t == 35 {
            increment += 33;
            t = self.read_huffman(&constants::MACROBLOCK_ADDRESS_INCREMENT);
        }
        increment += t;

        // Process skipped macroblocks
        if self.slice_beginning {
            // The first macroblock in the slice is relative to the previous row, we don't have to
            // handle any previous macroblocks
            self.slice_beginning = false;
            self.macroblock_address += increment as usize;
        } else {
            if increment > 1 {
                // Skipped macroblocks reset DC predictors
                // this.dcPredictorY  = 128;
                // this.dcPredictorCr = 128;
                // this.dcPredictorCb = 128;
    
                // Skipped macroblocks in P-pictures reset motion vectors
                // if (this.pictureType === MPEG1.PICTURE_TYPE.PREDICTIVE) {
                //     this.motionFwH = this.motionFwHPrev = 0;
                //     this.motionFwV = this.motionFwVPrev = 0;
                // }
            }
            
            while increment > 1 {
                self.macroblock_address += 1;
                self.mb_row = self.macroblock_address / self.mb_width as usize;
                self.mb_col = self.macroblock_address % self.mb_width as usize;
                // this.copyMacroblock(
                //     this.motionFwH, this.motionFwV,
                //     this.forwardY, this.forwardCr, this.forwardCb
                // );
                increment -= 1;
            }
            
            self.macroblock_address += 1;
        }
        
        self.mb_row = self.macroblock_address / self.mb_width as usize;
        self.mb_col = self.macroblock_address % self.mb_width as usize;
        let mb_table: &[i16] = if self.picture_type == constants::PICTURE_TYPE_INTRA { &constants::MACROBLOCK_TYPE_INTRA } else { &constants::MACROBLOCK_TYPE_PREDICTIVE };
        
        let macroblock_type = self.read_huffman(mb_table);
        let macroblock_intra = macroblock_type & 0b00001 != 0;
        let macroblock_mot_fw = macroblock_type & 0b01000 != 0;

        if macroblock_type & 0b10000 != 0 {
            let quantizer_scale = self.buffer[self.pointer..self.pointer+5].load::<u8>();
            self.pointer += 5;
        }
        
        if macroblock_intra {
            // Intra-coded macroblocks reset motion vectors
            // this.motionFwH = this.motionFwHPrev = 0;
            // this.motionFwV = this.motionFwVPrev = 0;
        } else {
            // Non-intra macroblocks reset DC predictors
            // 	this.dcPredictorY = 128;
            // 	this.dcPredictorCr = 128;
            // 	this.dcPredictorCb = 128;

            // 	this.decodeMotionVectors();
            // 	this.copyMacroblock(
            // 		this.motionFwH, this.motionFwV,
            // 		this.forwardY, this.forwardCr, this.forwardCb
            // 	);
            // }
        }

        // Decode blocks
        // var cbp = ((this.macroblockType & 0x02) !== 0)
        // ? this.readHuffman(MPEG1.CODE_BLOCK_PATTERN)
        // : (this.macroblockIntra ? 0x3f : 0);

        // for (var block = 0, mask = 0x20; block < 6; block++) {
        //     if ((cbp & mask) !== 0) {
        //         this.decodeBlock(block);
        //     }
        //     mask >>= 1;
        // }
    }
    
    fn read_huffman(&mut self, code_table: &[i16]) -> u16 {
        let mut state: i16 = 0;
        loop {
            let bit = self.buffer[self.pointer] as usize;
            self.pointer += 1;
            state = code_table[state as usize + bit];
            if state >= 0 && code_table[state as usize] != 0 { break }
        }
        code_table[state as usize + 2] as u16
    }
}

#[rustfmt::skip]
mod constants {
    pub const PICTURE_START_CODE: u32 = 0x00_00_01_00;
    pub const SLICE_FIRST_START_CODE: u32 = 0x00_00_01_01;
    pub const SLICE_LAST_START_CODE: u32 = 0x00_00_01_AF;
    pub const USER_DATA_START_CODE: u32 = 0x00_00_01_B5;
    pub const SEQUENCE_HEADER_CODE: u32 = 0x00_00_01_B3;
    pub const EXTENSION_START_CODE: u32 = 0x00_00_01_B5;

    pub const PICTURE_TYPE_INTRA: u8 = 0b001;
    pub const PICTURE_TYPE_PREDICTIVE: u8 = 0b010;
    
    pub const ZIG_ZAG: [usize; 64] = [
        0,  1,  8, 16,  9,  2,  3, 10,
       17, 24, 32, 25, 18, 11,  4,  5,
       12, 19, 26, 33, 40, 48, 41, 34,
       27, 20, 13,  6,  7, 14, 21, 28,
       35, 42, 49, 56, 57, 50, 43, 36,
       29, 22, 15, 23, 30, 37, 44, 51,
       58, 59, 52, 45, 38, 31, 39, 46,
       53, 60, 61, 54, 47, 55, 62, 63
   ];

    pub const DEFAULT_INTRA_QUANT_MATRIX: [u8; 64] = [
         8, 16, 19, 22, 26, 27, 29, 34,
        16, 16, 22, 24, 27, 29, 34, 37,
        19, 22, 26, 27, 29, 34, 34, 38,
        22, 22, 26, 27, 29, 34, 37, 40,
        22, 26, 27, 29, 32, 35, 40, 48,
        26, 27, 29, 32, 35, 40, 48, 58,
        26, 27, 29, 34, 38, 46, 56, 69,
        27, 29, 35, 38, 46, 56, 69, 83
    ];

    pub const DEFAULT_NON_INTRA_QUANT_MATRIX: [u8; 64] = [
        16, 16, 16, 16, 16, 16, 16, 16,
        16, 16, 16, 16, 16, 16, 16, 16,
        16, 16, 16, 16, 16, 16, 16, 16,
        16, 16, 16, 16, 16, 16, 16, 16,
        16, 16, 16, 16, 16, 16, 16, 16,
        16, 16, 16, 16, 16, 16, 16, 16,
        16, 16, 16, 16, 16, 16, 16, 16,
        16, 16, 16, 16, 16, 16, 16, 16
    ];
    
    pub const MACROBLOCK_ADDRESS_INCREMENT: [i16; 75*3] = [
        1*3,  2*3,  0, //   0
        3*3,  4*3,  0, //   1  0
          0,    0,  1, //   2  1.
        5*3,  6*3,  0, //   3  00
        7*3,  8*3,  0, //   4  01
        9*3, 10*3,  0, //   5  000
       11*3, 12*3,  0, //   6  001
          0,    0,  3, //   7  010.
          0,    0,  2, //   8  011.
       13*3, 14*3,  0, //   9  0000
       15*3, 16*3,  0, //  10  0001
          0,    0,  5, //  11  0010.
          0,    0,  4, //  12  0011.
       17*3, 18*3,  0, //  13  0000 0
       19*3, 20*3,  0, //  14  0000 1
          0,    0,  7, //  15  0001 0.
          0,    0,  6, //  16  0001 1.
       21*3, 22*3,  0, //  17  0000 00
       23*3, 24*3,  0, //  18  0000 01
       25*3, 26*3,  0, //  19  0000 10
       27*3, 28*3,  0, //  20  0000 11
         -1, 29*3,  0, //  21  0000 000
         -1, 30*3,  0, //  22  0000 001
       31*3, 32*3,  0, //  23  0000 010
       33*3, 34*3,  0, //  24  0000 011
       35*3, 36*3,  0, //  25  0000 100
       37*3, 38*3,  0, //  26  0000 101
          0,    0,  9, //  27  0000 110.
          0,    0,  8, //  28  0000 111.
       39*3, 40*3,  0, //  29  0000 0001
       41*3, 42*3,  0, //  30  0000 0011
       43*3, 44*3,  0, //  31  0000 0100
       45*3, 46*3,  0, //  32  0000 0101
          0,    0, 15, //  33  0000 0110.
          0,    0, 14, //  34  0000 0111.
          0,    0, 13, //  35  0000 1000.
          0,    0, 12, //  36  0000 1001.
          0,    0, 11, //  37  0000 1010.
          0,    0, 10, //  38  0000 1011.
       47*3,   -1,  0, //  39  0000 0001 0
         -1, 48*3,  0, //  40  0000 0001 1
       49*3, 50*3,  0, //  41  0000 0011 0
       51*3, 52*3,  0, //  42  0000 0011 1
       53*3, 54*3,  0, //  43  0000 0100 0
       55*3, 56*3,  0, //  44  0000 0100 1
       57*3, 58*3,  0, //  45  0000 0101 0
       59*3, 60*3,  0, //  46  0000 0101 1
       61*3,   -1,  0, //  47  0000 0001 00
         -1, 62*3,  0, //  48  0000 0001 11
       63*3, 64*3,  0, //  49  0000 0011 00
       65*3, 66*3,  0, //  50  0000 0011 01
       67*3, 68*3,  0, //  51  0000 0011 10
       69*3, 70*3,  0, //  52  0000 0011 11
       71*3, 72*3,  0, //  53  0000 0100 00
       73*3, 74*3,  0, //  54  0000 0100 01
          0,    0, 21, //  55  0000 0100 10.
          0,    0, 20, //  56  0000 0100 11.
          0,    0, 19, //  57  0000 0101 00.
          0,    0, 18, //  58  0000 0101 01.
          0,    0, 17, //  59  0000 0101 10.
          0,    0, 16, //  60  0000 0101 11.
          0,    0, 35, //  61  0000 0001 000. -- macroblock_escape
          0,    0, 34, //  62  0000 0001 111. -- macroblock_stuffing
          0,    0, 33, //  63  0000 0011 000.
          0,    0, 32, //  64  0000 0011 001.
          0,    0, 31, //  65  0000 0011 010.
          0,    0, 30, //  66  0000 0011 011.
          0,    0, 29, //  67  0000 0011 100.
          0,    0, 28, //  68  0000 0011 101.
          0,    0, 27, //  69  0000 0011 110.
          0,    0, 26, //  70  0000 0011 111.
          0,    0, 25, //  71  0000 0100 000.
          0,    0, 24, //  72  0000 0100 001.
          0,    0, 23, //  73  0000 0100 010.
          0,    0, 22  //  74  0000 0100 011.
   ];
    
    pub const MACROBLOCK_TYPE_INTRA: [i16; 3*4] = [
        1*3,  2*3,     0, //   0
         -1,  3*3,     0, //   1  0
          0,    0,  0x01, //   2  1.
          0,    0,  0x11  //   3  01.
    ];

    pub const MACROBLOCK_TYPE_PREDICTIVE: [i16; 3*14]  = [
        1*3,  2*3,     0, //  0
        3*3,  4*3,     0, //  1  0
          0,    0,  0x0a, //  2  1.
        5*3,  6*3,     0, //  3  00
          0,    0,  0x02, //  4  01.
        7*3,  8*3,     0, //  5  000
          0,    0,  0x08, //  6  001.
        9*3, 10*3,     0, //  7  0000
       11*3, 12*3,     0, //  8  0001
         -1, 13*3,     0, //  9  00000
          0,    0,  0x12, // 10  00001.
          0,    0,  0x1a, // 11  00010.
          0,    0,  0x01, // 12  00011.
          0,    0,  0x11  // 13  000001.
    ];
}
