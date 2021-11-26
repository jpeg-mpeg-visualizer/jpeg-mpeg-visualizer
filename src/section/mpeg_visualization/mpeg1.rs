use std::cell::{RefCell, RefMut};
use std::ops::Deref;
use std::rc::Rc;
use bitvec::prelude::*;

use self::constants::{DCT_DC_SIZE_CHROMINANCE, DCT_DC_SIZE_LUMINANCE};

enum FrameOrder {
    Forward,
    Backward
}

struct VideoMotion {
    pub full_pel: bool,
    pub r_size: u32,
    pub is_set: bool,
    pub h: i32,
    pub v: i32,
}

#[derive(Clone)]
pub struct FrameImage {
    pub y: Vec<u8>,
    pub cb: Vec<u8>,
    pub cr: Vec<u8>,
}

impl FrameImage {
    fn clear(&mut self) {
        let old_size_y = self.y.len();
        let old_size_cb = self.y.len();
        
        self.y.clear();
        self.cb.clear();
        self.cr.clear();
        self.y.resize(old_size_y, 0);
        self.cb.resize(old_size_cb, 0);
        self.cr.resize(old_size_cb, 0);
    }
}

impl Default for FrameImage {
    fn default() -> Self {
        FrameImage {
            y: vec![],
            cb: vec![],
            cr: vec![],
        }
    }
}

#[derive(Clone)]
pub struct VideoFrame {
    pub width: u16,
    pub height: u16,

    pub current: FrameImage,
    pub skipped: FrameImage,
    pub moved: FrameImage,
    pub intra: FrameImage,
}

impl Default for VideoFrame {
    fn default() -> Self {
        VideoFrame {
            width: 0,
            height: 0,

            current: Default::default(),
            skipped: Default::default(),
            moved: Default::default(),
            intra: Default::default(),
        }
    }
}

pub struct DecodedFrame {
    pub picture_type: u8,
    pub frame: VideoFrame,

    pub size: usize,
    pub macroblock_count: usize,
    pub block_count: usize,
    pub macroblock_info: Vec<MacroblockInfo>,
}

#[derive(Clone)]
pub struct MacroblockInfo {
    pub size: usize,
    pub encoded_blocks: MacroblockEncodedBlocks,
    pub kind: MacroblockInfoKind,
}

impl MacroblockInfo {
    fn skipped() -> MacroblockInfo {
        MacroblockInfo {
            size: 0,
            encoded_blocks: MacroblockEncodedBlocks::default(),
            kind: MacroblockInfoKind::Skipped,
        }
    }
}

#[derive(Default, Clone)]
pub struct MacroblockEncodedBlocks {
    pub blocks: [Option<Box<[i32; 64]>>; 6],
}

impl MacroblockEncodedBlocks {
    pub fn set_nth(&mut self, n: usize, value: &[i32; 64]) {
        self.blocks[n] = Some(Box::new(*value));
    }
}

#[derive(Debug, Clone)]
pub enum MacroblockInfoKind {
    Skipped,
    Moved { direction: (i32, i32) },
    Intra,
}

#[derive(Debug, Clone)]
enum MacroblockDestination {
    Current,
    Skipped,
    Moved,
}


#[derive(Clone)]
struct DecodingStats {
    pub picture_type: u8,
    pub size: usize,
    pub macroblock_count: usize,
    pub block_count: usize,

    pub macroblock_info: Vec<MacroblockInfo>,
}


impl Default for DecodingStats {
    fn default() -> Self {
        DecodingStats {
            picture_type: 0,
            size: 0,
            macroblock_count: 0,
            block_count: 0,
            macroblock_info: Default::default(),
        }
    }
}

pub struct MPEG1 {
    pointer: usize,
    buffer: BitVec<Msb0, u8>,
    has_sequence_header: bool,
    has_reference_frame: bool,

    width: u16,
    height: u16,
    mb_width: u16,
    mb_row: usize,
    mb_col: usize,
    coded_width: u32,

    picture_type: u8,

    frame_current: Rc<RefCell<VideoFrame>>,
    frame_forward: Rc<RefCell<VideoFrame>>,
    frame_backward: Rc<RefCell<VideoFrame>>,

    stats_next: DecodingStats,
    stats_current: DecodingStats,

    intra_quant_matrix: [u8; 64],
    non_intra_quant_matrix: [u8; 64],

    slice_beginning: bool,
    macroblock_address: i32,

    dc_predictor_y: u8,
    dc_predictor_cr: u8,
    dc_predictor_cb: u8,


    motion_forward: VideoMotion,
    motion_backward: VideoMotion,

    block_data: [i32; 64],
    quantizer_scale: u8,
    macroblock_count: usize,
    block_count: usize,
}

impl MPEG1 {
    pub fn from_bytes(bytes: Vec<u8>) -> MPEG1 {
        MPEG1 {
            pointer: 0,
            buffer: BitVec::from_vec(bytes),
            has_sequence_header: false,
            has_reference_frame: false,

            width: 0,
            height: 0,

            mb_width: 0,
            mb_row: 0,
            mb_col: 0,
            coded_width: 0,
            picture_type: 0,

            frame_current: Rc::<_>::default(),
            frame_forward: Rc::<_>::default(),
            frame_backward: Rc::<_>::default(),

            stats_next: Default::default(),
            stats_current: Default::default(),

            intra_quant_matrix: constants::DEFAULT_INTRA_QUANT_MATRIX,
            non_intra_quant_matrix: constants::DEFAULT_NON_INTRA_QUANT_MATRIX,
            slice_beginning: false,
            macroblock_address: 0,
            dc_predictor_y: 128,
            dc_predictor_cr: 128,
            dc_predictor_cb: 128,

            motion_forward: VideoMotion {
                full_pel: false,
                is_set: false,
                r_size: 0,
                h: 0,
                v: 0,
            },
            motion_backward: VideoMotion {
                full_pel: false,
                is_set: false,
                r_size: 0,
                h: 0,
                v: 0,
            },

            block_data: [0; 64],
            quantizer_scale: 0,
            macroblock_count: 0,
            block_count: 0,
        }
    }

    pub fn decode(&mut self) -> Option<DecodedFrame> {
        if !self.has_sequence_header {
            self.find_start_code(constants::SEQUENCE_HEADER_CODE);
            self.decode_sequence_header();
        }
        
        let mut frame: Option<Rc<RefCell<VideoFrame>>> = None;
        loop {
            self.stats_current = self.stats_next.clone();
            self.stats_next.macroblock_info = vec![];

            if self.find_start_code(constants::PICTURE_START_CODE) {
                self.decode_picture();
            } else {
                if self.has_reference_frame {
                    self.has_reference_frame = false;
                    frame = Some(self.frame_backward.clone());
                    break;
                } else {
                    return None
                }
            }

            if self.picture_type == constants::PICTURE_TYPE_B {
                frame = Some(self.frame_current.clone());
            } else if self.has_reference_frame {
                frame = Some(self.frame_forward.clone());
            } else {
                self.has_reference_frame = true;
            }

            if frame.is_some() {
                break;
            }

        }

        Some(DecodedFrame {
            picture_type: self.stats_current.picture_type,
            frame: (*frame.unwrap()).borrow_mut().clone(),
            size: self.stats_current.size,
            macroblock_count: self.stats_current.macroblock_count,
            block_count: self.stats_current.block_count,
            macroblock_info: self.stats_current.macroblock_info.clone(),
        })
    }

    fn get_next_start_code(&mut self) -> Option<u32> {
        // byte align the pointer
        self.pointer = ((self.pointer + 7) / 8) * 8;
        while self.pointer + 32 < self.buffer.len() {
            let four_bytes = self.buffer[self.pointer..self.pointer + 32].load_be::<u32>();
            if (four_bytes & 0xFF_FF_FF_00) == 0x00_00_01_00 {
                return four_bytes.into();
            }
            self.pointer += 8;
        }
        None
    }

    fn find_start_code(&mut self, code: u32) -> bool {
        loop {
            match self.get_next_start_code() {
                Some(next_code) if next_code == code => return true,
                None => return false,
                Some(_) => {
                    self.pointer += 32;
                }
            }
        }
    }

    fn next_bytes_are_start_code(&mut self) -> Option<bool> {
        let aligned_pointer = ((self.pointer + 7) / 8) * 8;
        if aligned_pointer + 24 < self.buffer.len() {
            Some(self.buffer[aligned_pointer..aligned_pointer + 24].load_be::<u32>() == 0x00_00_01)
        } else {
            None
        }
    }

    fn init_buffers(&mut self, width: u16, height: u16) {
        self.mb_width = (width + 15) / 16;
        let mb_height = (height + 15) / 16;

        self.coded_width = self.mb_width as u32 * 16;
        let coded_height = mb_height as u32 * 16;
        let coded_size = self.coded_width * coded_height;

        Self::init_frame(width, height, coded_size, (*self.frame_current).borrow_mut());
        Self::init_frame(width, height, coded_size, (*self.frame_forward).borrow_mut());
        Self::init_frame(width, height, coded_size, (*self.frame_backward).borrow_mut());
    }

    fn init_frame(width: u16, height: u16, coded_size: u32, mut frame: RefMut<VideoFrame>) {
        frame.width = width;
        frame.height = height;

        frame.current.y = vec![0; coded_size as usize];
        frame.current.cb = vec![0; coded_size as usize / 4];
        frame.current.cr = vec![0; coded_size as usize / 4];

        frame.skipped.y = vec![0; coded_size as usize];
        frame.skipped.cb = vec![0; coded_size as usize / 4];
        frame.skipped.cr = vec![0; coded_size as usize / 4];

        frame.moved.y = vec![0; coded_size as usize];
        frame.moved.cb = vec![0; coded_size as usize / 4];
        frame.moved.cr = vec![0; coded_size as usize / 4];

        frame.intra.y = vec![0; coded_size as usize];
        frame.intra.cb = vec![0; coded_size as usize / 4];
        frame.intra.cr = vec![0; coded_size as usize / 4];
    }

    fn decode_sequence_header(&mut self) {
        // Skip over sequence header start code
        self.pointer += 32;

        let width = self.buffer[self.pointer..self.pointer + 12].load_be::<u16>();
        let height = self.buffer[self.pointer + 12..self.pointer + 12 + 12].load_be::<u16>();

        // Skip over 39 bits of data:
        // Pel aspect ratio - 4 bits
        // Picture rate - 4 bits
        // Bit rate - 18 bits
        // Marker bit - 1 bit
        // Vbv buffer size - 10 bit
        // Constrained parameters flag - 1 bit
        self.pointer += 12 + 12 + 4 + 4 + 18 + 1 + 10 + 1;

        if (width, height) != (self.width, self.height) {
            self.width = width;
            self.height = height;

            self.init_buffers(width, height);
        }

        let load_intra_quantizer_matrix =
            self.buffer[self.pointer..self.pointer + 1].load_be::<u8>();
        self.pointer += 1;

        if load_intra_quantizer_matrix == 1 {
            for i in 0..64 {
                self.intra_quant_matrix[constants::ZIG_ZAG[i]] =
                    self.buffer[self.pointer..self.pointer + 8].load_be::<u8>();
                self.pointer += 8;
            }
        }

        let load_non_intra_quantizer_matrix =
            self.buffer[self.pointer..self.pointer + 1].load_be::<u8>();
        self.pointer += 1;

        if load_non_intra_quantizer_matrix == 1 {
            for i in 0..64 {
                self.non_intra_quant_matrix[constants::ZIG_ZAG[i]] =
                    self.buffer[self.pointer..self.pointer + 8].load_be::<u8>();
                self.pointer += 8;
            }
        }

        self.has_sequence_header = true;
    }

    fn decode_picture(&mut self) {
        let old_pointer = self.pointer;

        // Skip over picture start code
        self.pointer += 32;

        // Skip over temporal reference
        self.pointer += 10;

        self.picture_type = self.buffer[self.pointer..self.pointer + 3].load_be::<u8>();

        // Skip over VBV buffer delay
        self.pointer += 3 + 16;

        // Assert that the picture type is not D
        assert!(
            self.picture_type == constants::PICTURE_TYPE_INTRA
                || self.picture_type == constants::PICTURE_TYPE_PREDICTIVE
                || self.picture_type == constants::PICTURE_TYPE_B
        );

        if self.picture_type == constants::PICTURE_TYPE_PREDICTIVE
            || self.picture_type == constants::PICTURE_TYPE_B {
            self.motion_forward.full_pel = self.buffer[self.pointer];

            let forward_f_code =
                self.buffer[self.pointer + 1..self.pointer + 1 + 3].load_be::<u8>();
            self.pointer += 4;

            self.motion_forward.r_size = forward_f_code as u32 - 1;
        }

        if self.picture_type == constants::PICTURE_TYPE_B {
            self.motion_backward.full_pel = self.buffer[self.pointer];

            let backward_f_code =
                self.buffer[self.pointer + 1..self.pointer + 1 + 3].load_be::<u8>();
            self.pointer += 4;

            self.motion_backward.r_size = backward_f_code as u32 - 1;
        }

        let temp_frame = self.frame_forward.clone();

        if self.picture_type == constants::PICTURE_TYPE_INTRA
            || self.picture_type == constants::PICTURE_TYPE_PREDICTIVE {
            self.frame_forward = self.frame_backward.clone();
        }

        let mut start_code: Option<u32> = self.get_next_start_code();
        while let Some(constants::USER_DATA_START_CODE | constants::EXTENSION_START_CODE) =
            start_code
        {
            self.pointer += 32;
            start_code = self.get_next_start_code();
        }
        
        {
            let mut frame_current: RefMut<_> = self.frame_current.deref().borrow_mut();
            frame_current.moved.clear();
            frame_current.skipped.clear();
            frame_current.intra.clear();
        }

        while let Some(constants::SLICE_FIRST_START_CODE..=constants::SLICE_LAST_START_CODE) =
            start_code
        {
            self.decode_slice((start_code.unwrap() & 0x00_00_00_FF) as u16);
            start_code = self.get_next_start_code()
        }

        self.stats_next.picture_type = self.picture_type;
        self.stats_next.block_count = self.block_count;
        self.stats_next.macroblock_count = self.macroblock_count;
        self.stats_next.size = self.pointer - old_pointer;

        self.macroblock_count = 0;
        self.block_count = 0;

        self.macroblock_count = 0;
        self.block_count = 0;

        if self.picture_type == constants::PICTURE_TYPE_INTRA
            || self.picture_type == constants::PICTURE_TYPE_PREDICTIVE {
            self.frame_backward = self.frame_current.clone();
            self.frame_current = temp_frame.clone();
        }
    }

    fn decode_slice(&mut self, slice: u16) {
        self.slice_beginning = true;

        // Skip over slice start code
        self.pointer += 32;

        self.macroblock_address = ((slice - 1) * self.mb_width) as i32 - 1;

        self.motion_forward.h = 0;
        self.motion_forward.v = 0;

        self.motion_backward.h = 0;
        self.motion_backward.v = 0;

        self.dc_predictor_y = 128;
        self.dc_predictor_cr = 128;
        self.dc_predictor_cb = 128;

        self.quantizer_scale = self.buffer[self.pointer..self.pointer + 5].load_be::<u8>();
        self.pointer += 5;

        // Skip over extra information
        while self.buffer[self.pointer] {
            self.pointer += 1 + 8;
        }
        self.pointer += 1;

        // There must be at least one macroblock
        loop {
            self.decode_macroblock();
            self.macroblock_count += 1;
            if self.next_bytes_are_start_code() != Some(false) {
                break;
            };
        }

        for _ in 0..(self.mb_width as i32 - self.macroblock_address % self.mb_width as i32 - 1) {
            self.stats_next.macroblock_info.push(MacroblockInfo::skipped());
        }
    }

    fn decode_macroblock(&mut self) {
        let old_pointer = self.pointer;

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
            self.macroblock_address += increment;

            for _ in 0..(increment - 1) {
                self.stats_next.macroblock_info.push(MacroblockInfo::skipped());
            }
        } else {
            if increment > 1 {
                // Skipped macroblocks reset DC predictors
                self.dc_predictor_y = 128;
                self.dc_predictor_cr = 128;
                self.dc_predictor_cb = 128;

                // Skipped macroblocks in P-pictures reset motion vectors
                if self.picture_type == constants::PICTURE_TYPE_PREDICTIVE {
                    self.motion_forward.h = 0;
                    self.motion_forward.v = 0;
                }
            }

            while increment > 1 {
                self.macroblock_address += 1;
                self.mb_row = self.macroblock_address as usize / self.mb_width as usize;
                self.mb_col = self.macroblock_address as usize % self.mb_width as usize;
                self.predict_macroblock(MacroblockDestination::Current);
                self.predict_macroblock(MacroblockDestination::Skipped);
                increment -= 1;
                self.stats_next.macroblock_info.push(MacroblockInfo::skipped());
            }

            self.macroblock_address += 1;
        }

        self.mb_row = self.macroblock_address as usize / self.mb_width as usize;
        self.mb_col = self.macroblock_address as usize % self.mb_width as usize;
        let mb_table: &[i32] = match self.picture_type {
            constants::PICTURE_TYPE_INTRA => &constants::MACROBLOCK_TYPE_INTRA,
            constants::PICTURE_TYPE_PREDICTIVE => &constants::MACROBLOCK_TYPE_PREDICTIVE,
            _ => &constants::MACROBLOCK_TYPE_B
        };

        let macroblock_type = self.read_huffman(mb_table);
        let macroblock_intra = macroblock_type & 0b00001 != 0;

        self.motion_forward.is_set = macroblock_type & 0b01000 != 0;
        self.motion_backward.is_set = macroblock_type & 0b00100 != 0;

        if macroblock_type & 0b10000 != 0 {
            self.quantizer_scale = self.buffer[self.pointer..self.pointer + 5].load_be::<u8>();
            self.pointer += 5;
        }

        if macroblock_intra {
            // Intra-coded macroblocks reset motion vectors
            self.motion_forward.h = 0;
            self.motion_forward.v = 0;
            self.motion_backward.h = 0;
            self.motion_backward.v = 0;

            self.stats_next.macroblock_info.push(MacroblockInfo {
                size: 0,
                encoded_blocks: MacroblockEncodedBlocks::default(),
                kind: MacroblockInfoKind::Intra,
            });
        } else {
            // Non-intra macroblocks reset DC predictors
            self.dc_predictor_y = 128;
            self.dc_predictor_cr = 128;
            self.dc_predictor_cb = 128;

            self.decode_motion_vectors();
            self.predict_macroblock(
                MacroblockDestination::Current,
            );
            self.predict_macroblock(
                MacroblockDestination::Moved,
            );
            self.stats_next.macroblock_info.push(MacroblockInfo {
                size: 0,
                encoded_blocks: MacroblockEncodedBlocks::default(),
                kind: MacroblockInfoKind::Moved {
                    direction: (self.motion_forward.h, self.motion_forward.v),
                },
            });
        }

        // Decode blocks
        let cbp = if (macroblock_type & 0b00010) != 0 {
            self.read_huffman(&constants::CODE_BLOCK_PATTERN)
        } else if macroblock_intra {
            0b111111
        } else {
            0
        };

        for block in 0..6 {
            let mask = 0b100000 >> block;
            if cbp & mask != 0 {
                self.decode_block(block, macroblock_intra);
                self.block_count += 1;
            }
        }

        self.stats_next.macroblock_info[self.macroblock_address as usize].size = self.pointer - old_pointer;
    }

    fn decode_motion_vectors(&mut self) {
        if self.motion_forward.is_set {
            let r_size = self.motion_forward.r_size;
            self.motion_forward.h = self.decode_motion_vector(r_size, self.motion_forward.h);
            self.motion_forward.v = self.decode_motion_vector(r_size, self.motion_forward.v);

        } else if self.picture_type == constants::PICTURE_TYPE_PREDICTIVE {
            self.motion_forward.h = 0;
            self.motion_forward.v = 0;
        }

        if self.motion_backward.is_set {
            let r_size = self.motion_backward.r_size;
            self.motion_backward.h = self.decode_motion_vector(r_size, self.motion_backward.h);
            self.motion_backward.v = self.decode_motion_vector(r_size, self.motion_backward.v);
        }
    }

    fn decode_motion_vector(&mut self, r_size: u32, motion: i32) -> i32 {
        let f_scale = 1 << r_size;

        let mut d: i32;
        let mut new_motion = motion;

        let code = self.read_huffman(&constants::MOTION) as i32;
        if code != 0 && f_scale != 1 {
            let r = self.buffer[self.pointer..self.pointer + r_size as usize]
                .load_be::<u32>();
            self.pointer += r_size as usize;
            d = ((code.abs() - 1) << r_size) + r as i32 + 1;
            if code < 0 {
                d = -d;
            }
        } else {
            d = code;
        }

        new_motion += d;
        if new_motion > ((f_scale as i32) << 4) - 1 {
            new_motion -= (f_scale as i32) << 5;
        } else if new_motion < (-(f_scale as i32) << 4) {
            new_motion += (f_scale as i32) << 5;
        }

        new_motion
    }

    fn predict_macroblock(&mut self, destination: MacroblockDestination) {
        let mut forward_h = self.motion_forward.h;
        let mut forward_v = self.motion_forward.v;

        if self.motion_forward.full_pel {
            forward_h = forward_h << 1;
            forward_v = forward_v << 1;
        }

        if self.picture_type == constants::PICTURE_TYPE_B {
            let mut backward_h = self.motion_backward.h;
            let mut backward_v = self.motion_backward.v;

            if self.motion_backward.full_pel {
                backward_h = backward_h << 1;
                backward_v = backward_v << 1;
            }

            if self.motion_forward.is_set {
                self.copy_macroblock(forward_h, forward_v, destination.clone(), FrameOrder::Forward);
                if self.motion_backward.is_set {
                    self.interpolate_macroblock(backward_h, backward_v, destination);
                }
            }
            else {
                self.copy_macroblock(backward_h, backward_v, destination, FrameOrder::Backward);
            }
        } else {
            self.copy_macroblock(forward_h, forward_v, destination, FrameOrder::Forward);
        }
    }

    fn decode_block(&mut self, block: u16, macroblock_intra: bool) {
        let mut n = 0;
        let quant_matrix;

        if macroblock_intra {
            let (predictor, dct_size) = if block < 4 {
                // Is luminance block
                (
                    self.dc_predictor_y,
                    self.read_huffman(&DCT_DC_SIZE_LUMINANCE),
                )
            } else {
                (
                    if block == 4 {
                        self.dc_predictor_cr
                    } else {
                        self.dc_predictor_cb
                    },
                    self.read_huffman(&DCT_DC_SIZE_CHROMINANCE),
                )
            };

            if dct_size > 0 {
                let differential =
                    self.buffer[self.pointer..self.pointer + dct_size as usize].load_be::<u8>();
                self.pointer += dct_size as usize;
                if differential & (1 << (dct_size - 1)) != 0 {
                    self.block_data[0] = (predictor + differential) as i32;
                } else {
                    self.block_data[0] =
                        predictor as i32 + ((-1 << dct_size as i32) | (differential as i32 + 1));
                }
            } else {
                self.block_data[0] = predictor as i32;
            }

            if block < 4 {
                self.dc_predictor_y = self.block_data[0] as u8;
            } else if block == 4 {
                self.dc_predictor_cr = self.block_data[0] as u8;
            } else {
                self.dc_predictor_cb = self.block_data[0] as u8;
            }

            self.block_data[0] <<= 8;
            quant_matrix = self.intra_quant_matrix;
            n = 1;
        } else {
            quant_matrix = self.non_intra_quant_matrix;
        }

        let mut level: i32;
        loop {
            let run: u8;
            let coeff = self.read_huffman(&constants::DCT_COEFF);

            let should_break = if coeff == 0x0001 && n > 0 {
                self.pointer += 1;
                !self.buffer[self.pointer - 1]
            } else {
                false
            };

            if should_break {
                break;
            }
            if coeff == 0xffff {
                run = self.buffer[self.pointer..self.pointer + 6].load_be::<u8>();
                level = self.buffer[self.pointer + 6..self.pointer + 6 + 8].load_be::<u8>() as i32;
                self.pointer += 6 + 8;

                if level == 0 {
                    level = self.buffer[self.pointer..self.pointer + 8].load_be::<u8>() as i32;
                    self.pointer += 8;
                } else if level == 128 {
                    let tmp = self.buffer[self.pointer..self.pointer + 8].load_be::<u8>() as i32;
                    self.pointer += 8;
                    level = tmp - 256;
                } else if level > 128 {
                    level -= 256;
                }
            } else {
                run = (coeff >> 8) as u8;
                level = coeff as i32 & 0xff;
                if self.buffer[self.pointer] {
                    level = -level;
                }
                self.pointer += 1;
            }

            n += run;
            let de_zig_zagged = constants::ZIG_ZAG[n as usize];
            n += 1;

            level <<= 1;
            if !macroblock_intra {
                level += if level < 0 { -1 } else { 1 };
            }
            level = (level * self.quantizer_scale as i32 * quant_matrix[de_zig_zagged] as i32) / 16;
            if level & 1 == 0 {
                level -= if level < 0 { -1 } else { 1 };
            }
            level = level.clamp(-2048, 2048);

            self.block_data[de_zig_zagged] =
                level * constants::PREMULTIPLIER_MATRIX[de_zig_zagged] as i32;
        }

        let (mut dest_array, mut dest_array_new);
        let mut dest_index;
        let scan;

        if block < 4 {
            let temp = RefMut::map_split((*self.frame_current).borrow_mut(), |c|
                (
                    &mut c.current.y,
                    if macroblock_intra {
                        &mut c.intra.y
                    } else {
                        &mut c.moved.y
                    }
                )
            );
            dest_array = temp.0;
            dest_array_new = temp.1;
            scan = self.coded_width as usize - 8;
            dest_index = (self.mb_row * self.coded_width as usize + self.mb_col) * 16;
            if block & 1 != 0 {
                dest_index += 8;
            }
            if block & 2 != 0 {
                dest_index += self.coded_width as usize * 8;
            }
        } else {
            if block == 4 {
                let temp = RefMut::map_split((*self.frame_current).borrow_mut(), |c|
                    (
                        &mut c.current.cb,
                        if macroblock_intra {
                            &mut c.intra.cb
                        } else {
                             &mut c.moved.cb
                        }
                    )
                );
                dest_array = temp.0;
                dest_array_new = temp.1;
            } else {
                let temp = RefMut::map_split((*self.frame_current).borrow_mut(), |c|
                    (
                        &mut c.current.cr,
                        if macroblock_intra {
                            &mut c.intra.cr
                        } else {
                            &mut c.moved.cr
                        }
                    )
                );
                dest_array = temp.0;
                dest_array_new = temp.1;
            };
            scan = self.coded_width as usize / 2 - 8;
            dest_index = ((self.mb_row * self.coded_width as usize) * 4) + self.mb_col * 8;
        }

        if macroblock_intra {
            // Overwrite
            if n == 1 {
                let value = ((self.block_data[0] + 128) >> 8) as u8;
                copy_value_to_destination(value, &mut dest_array, dest_index, scan);
                copy_value_to_destination(value, &mut dest_array_new, dest_index, scan);
                self.stats_next.macroblock_info[self.macroblock_address as usize]
                    .encoded_blocks
                    .set_nth(block.into(), &[value.into(); 64]);
                self.block_data[0] = 0;
            } else {
                IDCT(&mut self.block_data);
                copy_block_to_destination(&self.block_data, &mut dest_array, dest_index, scan);
                copy_block_to_destination(&self.block_data, &mut dest_array_new, dest_index, scan);
                self.stats_next.macroblock_info[self.macroblock_address as usize]
                    .encoded_blocks
                    .set_nth(block.into(), &self.block_data);
                self.block_data = [0; 64];
            }
        } else {
            if n == 1 {
                let value = (self.block_data[0] + 128) >> 8;
                add_value_to_destination(value, &mut dest_array, dest_index, scan);
                add_value_to_destination(value, &mut dest_array_new, dest_index, scan);
                self.stats_next.macroblock_info[self.macroblock_address as usize]
                    .encoded_blocks
                    .set_nth(block.into(), &[value as i32; 64]);
                self.block_data[0] = 0;
            } else {
                IDCT(&mut self.block_data);
                add_block_to_destination(&self.block_data, &mut dest_array, dest_index, scan);
                add_block_to_destination(&self.block_data, &mut dest_array_new, dest_index, scan);
                self.stats_next.macroblock_info[self.macroblock_address as usize]
                    .encoded_blocks
                    .set_nth(block.into(), &self.block_data);
                self.block_data = [0; 64];
            }
        }
    }

    fn copy_macroblock(
        &mut self,
        motion_h: i32,
        motion_v: i32,
        destination: MacroblockDestination,
        s_order: FrameOrder
    ) {
        let s = match s_order{
            FrameOrder::Forward => self.frame_forward.borrow(),
            FrameOrder::Backward => self.frame_backward.borrow(),
        };
        let  (s_y, s_cr, s_cb) = (&s.current.y, &s.current.cr, &s.current.cb);

        let mut d: RefMut<_> = (*self.frame_current).borrow_mut();
        let (d_frame) = match destination {
            MacroblockDestination::Current => (
                &mut d.current
            ),
            MacroblockDestination::Skipped => (
                &mut d.skipped
            ),
            MacroblockDestination::Moved => (
                &mut d.moved
            )
        };

        // Luminance
        let width = self.coded_width as usize;
        let scan = width - 16;

        let h = motion_h as isize >> 1;
        let v = motion_v as isize >> 1;

        let is_motion_h_odd = motion_h & 1 == 1;
        let is_motion_v_odd = motion_v & 1 == 1;

        let mut src = (((self.mb_row as isize * 16) + v) * width as isize
            + (self.mb_col as isize * 16)
            + h) as usize;
        let mut dest = (self.mb_row * width + self.mb_col) * 16;
        let last = dest + (width * 16);

        if is_motion_h_odd {
            if is_motion_v_odd {
                while dest < last {
                    for _x in 0..16 {
                        // vertical motion is half pel (?) so we have to compute average of above pixel
                        let mut sum: u16 = s_y[src] as u16;
                        sum += s_y[src + 1] as u16;
                        sum += s_y[src + width] as u16;
                        sum += s_y[src + width + 1] as u16;
                        sum += 2;

                        d_frame.y[dest] = (sum >> 2) as u8;
                        dest += 1;
                        src += 1;
                    }
                    dest += scan;
                    src += scan;
                }
            } else {
                while dest < last {
                    for _x in 0..16 {
                        let mut sum: u16 = s_y[src] as u16;
                        sum += s_y[src + 1] as u16;
                        sum += 1 as u16;

                        d_frame.y[dest] =  (sum >> 1) as u8;
                        dest += 1;
                        src += 1;
                    }
                    dest += scan;
                    src += scan;
                }
            }
        } else {
            if is_motion_v_odd {
                while dest < last {
                    for _x in 0..16 {
                        let mut sum: u16 = s_y[src] as u16;
                        sum += s_y[src + width] as u16;
                        sum += 1;

                        d_frame.y[dest] = (sum >> 1) as u8;
                        dest += 1;
                        src += 1;
                    }
                    dest += scan;
                    src += scan;
                }
            } else {
                while dest < last {
                    for _x in 0..16 {
                        d_frame.y[dest] = s_y[src];
                        dest += 1;
                        src += 1;
                    }
                    dest += scan;
                    src += scan;
                }
            }
        }

        // Chrominance
        let width = self.mb_width as usize * 8;
        let scan = width - 8;

        let h = (motion_h as isize / 2) >> 1;
        let v = (motion_v as isize / 2) >> 1;

        let is_motion_h_odd = (motion_h / 2) & 1 == 1;
        let is_motion_v_odd = (motion_v / 2) & 1 == 1;

        let mut src = (((self.mb_row as isize * 8) + v) * width as isize
            + (self.mb_col as isize * 8)
            + h) as usize;
        let mut dest = (self.mb_row * width + self.mb_col) * 8;
        let last = dest + (width * 8);

        if is_motion_h_odd {
            if is_motion_v_odd {
                while dest < last {
                    for _x in 0..8 {
                        // vertical motion is half pel (?) so we have to compute average of above pixel
                        let mut sum = s_cr[src] as u16;
                        sum += s_cr[src + 1] as u16;
                        sum += s_cr[src + width] as u16;
                        sum += s_cr[src + width + 1] as u16;
                        sum += 2;

                        d_frame.cr[dest] = (sum >> 2) as u8;

                        let mut sum = s_cb[src] as u16;
                        sum += s_cb[src + 1] as u16;
                        sum += s_cb[src + width] as u16;
                        sum += s_cb[src + width + 1] as u16;
                        sum += 2;

                        d_frame.cb[dest] = (sum >> 2) as u8;

                        dest += 1;
                        src += 1;
                    }
                    dest += scan;
                    src += scan;
                }
            } else {
                while dest < last {
                    for _x in 0..8 {
                        let mut sum = s_cr[src] as u16;
                        sum += s_cr[src + 1] as u16;
                        sum += 1;

                        d_frame.cr[dest] = (sum >> 1) as u8;

                        let mut sum = s_cb[src] as u16 + 1;
                        sum += s_cb[src + 1] as u16;
                        sum += 1;

                        d_frame.cb[dest] = (sum >> 1) as u8;

                        dest += 1;
                        src += 1;
                    }
                    dest += scan;
                    src += scan;
                }
            }
        } else {
            if is_motion_v_odd {
                while dest < last {
                    for _x in 0..8 {
                        let mut sum = s_cr[src] as u16 + 1;
                        sum += s_cr[src + width] as u16;
                        sum += 1;

                        d_frame.cr[dest] = (sum >> 1) as u8;

                        let mut sum = s_cb[src] as u16;
                        sum += s_cb[src + width] as u16;
                        sum += 1;

                        d_frame.cb[dest] = (sum >> 1) as u8;

                        dest += 1;
                        src += 1;
                    }
                    dest += scan;
                    src += scan;
                }
            } else {
                while dest < last {
                    for _x in 0..8 {
                        d_frame.cr[dest] = s_cr[src];
                        d_frame.cb[dest] = s_cb[src];
                        dest += 1;
                        src += 1;
                    }
                    dest += scan;
                    src += scan;
                }
            }
        }
    }

    fn interpolate_macroblock(
        &mut self,
        motion_h: i32,
        motion_v: i32,
        destination: MacroblockDestination,
    ) {
        let s = self.frame_backward.borrow();
        let  (s_y, s_cr, s_cb) = (&s.current.y, &s.current.cr, &s.current.cb);

        let mut d: RefMut<_> = (*self.frame_current).borrow_mut();
        let (d_frame) = match destination {
            MacroblockDestination::Current => (
                &mut d.current
            ),
            MacroblockDestination::Skipped => (
                &mut d.skipped
            ),
            MacroblockDestination::Moved => (
                &mut d.moved
            )
        };

        // Luminance
        let width = self.coded_width as usize;
        let scan = width - 16;

        let h = motion_h as isize >> 1;
        let v = motion_v as isize >> 1;

        let is_motion_h_odd = motion_h & 1 == 1;
        let is_motion_v_odd = motion_v & 1 == 1;

        let mut src = (((self.mb_row as isize * 16) + v) * width as isize
            + (self.mb_col as isize * 16)
            + h) as usize;
        let mut dest = (self.mb_row * width + self.mb_col) * 16;
        let last = dest + (width * 16);

        if is_motion_h_odd {
            if is_motion_v_odd {
                while dest < last {
                    for _x in 0..16 {
                        // vertical motion is half pel (?) so we have to compute average of above pixel
                        let mut sum: u16 = s_y[src] as u16;
                        sum += s_y[src + 1] as u16;
                        sum += s_y[src + width] as u16;
                        sum += s_y[src + width + 1] as u16;
                        sum += 2;
                        sum >>= 2;
                        sum += 1;

                        sum += d_frame.y[dest] as u16;

                        d_frame.y[dest] = (sum >> 1) as u8;
                        dest += 1;
                        src += 1;
                    }
                    dest += scan;
                    src += scan;
                }
            } else {
                while dest < last {
                    for _x in 0..16 {
                        let mut sum: u16 = s_y[src] as u16;
                        sum += s_y[src + 1] as u16;
                        sum += 1 as u16;
                        sum >>= 1;
                        sum += 1;

                        sum += d_frame.y[dest] as u16;

                        d_frame.y[dest] =  (sum >> 1) as u8;
                        dest += 1;
                        src += 1;
                    }
                    dest += scan;
                    src += scan;
                }
            }
        } else {
            if is_motion_v_odd {
                while dest < last {
                    for _x in 0..16 {
                        let mut sum: u16 = s_y[src] as u16;
                        sum += s_y[src + width] as u16;
                        sum += 1;
                        sum >>= 1;
                        sum += 1;

                        sum += d_frame.y[dest] as u16;

                        d_frame.y[dest] = (sum >> 1) as u8;
                        dest += 1;
                        src += 1;
                    }
                    dest += scan;
                    src += scan;
                }
            } else {
                while dest < last {
                    for _x in 0..16 {
                        let mut sum: u16 = s_y[src] as u16;
                        sum += 1;

                        sum += d_frame.y[dest] as u16;

                        d_frame.y[dest] += (sum >> 1) as u8;
                        dest += 1;
                        src += 1;
                    }
                    dest += scan;
                    src += scan;
                }
            }
        }

        // Chrominance
        let width = self.mb_width as usize * 8;
        let scan = width - 8;

        let h = (motion_h as isize / 2) >> 1;
        let v = (motion_v as isize / 2) >> 1;

        let is_motion_h_odd = (motion_h / 2) & 1 == 1;
        let is_motion_v_odd = (motion_v / 2) & 1 == 1;

        let mut src = (((self.mb_row as isize * 8) + v) * width as isize
            + (self.mb_col as isize * 8)
            + h) as usize;
        let mut dest = (self.mb_row * width + self.mb_col) * 8;
        let last = dest + (width * 8);

        if is_motion_h_odd {
            if is_motion_v_odd {
                while dest < last {
                    for _x in 0..8 {
                        // vertical motion is half pel (?) so we have to compute average of above pixel
                        let mut sum = s_cr[src] as u16;
                        sum += s_cr[src + 1] as u16;
                        sum += s_cr[src + width] as u16;
                        sum += s_cr[src + width + 1] as u16;
                        sum += 2;
                        sum >>= 2;
                        sum += 1;

                        sum += d_frame.cr[dest] as u16;

                        d_frame.cr[dest] = (sum >> 1) as u8;

                        let mut sum = s_cb[src] as u16;
                        sum += s_cb[src + 1] as u16;
                        sum += s_cb[src + width] as u16;
                        sum += s_cb[src + width + 1] as u16;
                        sum += 2;
                        sum >>= 2;
                        sum += 1;

                        sum += d_frame.cb[dest] as u16;

                        d_frame.cb[dest] = (sum >> 1) as u8;

                        dest += 1;
                        src += 1;
                    }
                    dest += scan;
                    src += scan;
                }
            } else {
                while dest < last {
                    for _x in 0..8 {
                        let mut sum = s_cr[src] as u16;
                        sum += s_cr[src + 1] as u16;
                        sum += 1;
                        sum >>= 1;
                        sum += 1;

                        sum += d_frame.cr[dest] as u16;

                        d_frame.cr[dest] = (sum >> 1) as u8;

                        let mut sum = s_cb[src] as u16 + 1;
                        sum += s_cb[src + 1] as u16;
                        sum += 1;
                        sum >>= 1;
                        sum += 1;

                        sum += d_frame.cb[dest] as u16;

                        d_frame.cb[dest] = (sum >> 1) as u8;

                        dest += 1;
                        src += 1;
                    }
                    dest += scan;
                    src += scan;
                }
            }
        } else {
            if is_motion_v_odd {
                while dest < last {
                    for _x in 0..8 {
                        let mut sum = s_cr[src] as u16 + 1;
                        sum += s_cr[src + width] as u16;
                        sum += 1;
                        sum >>= 1;
                        sum += 1;

                        sum += d_frame.cr[dest] as u16;

                        d_frame.cr[dest] = (sum >> 1) as u8;

                        let mut sum = s_cb[src] as u16;
                        sum += s_cb[src + width] as u16;
                        sum += 1;
                        sum >>= 1;
                        sum += 1;

                        sum += d_frame.cb[dest] as u16;

                        d_frame.cb[dest] = (sum >> 1) as u8;

                        dest += 1;
                        src += 1;
                    }
                    dest += scan;
                    src += scan;
                }
            } else {
                while dest < last {
                    for _x in 0..8 {
                        d_frame.cr[dest] += s_cr[src] + 1;
                        d_frame.cb[dest] += s_cb[src] + 1;

                        d_frame.cr[dest] >>= 1;
                        d_frame.cb[dest] >>= 1;
                        dest += 1;
                        src += 1;
                    }
                    dest += scan;
                    src += scan;
                }
            }
        }
    }

    fn read_huffman(&mut self, code_table: &[i32]) -> i32 {
        let mut state: i32 = 0;
        loop {
            let bit = self.buffer[self.pointer] as usize;
            self.pointer += 1;
            state = code_table[state as usize + bit];
            if state < 0 || code_table[state as usize] == 0 {
                break;
            }
        }
        code_table[state as usize + 2]
    }
}

#[allow(clippy::identity_op)]
fn copy_block_to_destination(block: &[i32; 64], dest: &mut [u8], mut index: usize, scan: usize) {
    for n in (0..64).step_by(8) {
        dest[index + 0] = block[n + 0] as u8;
        dest[index + 1] = block[n + 1] as u8;
        dest[index + 2] = block[n + 2] as u8;
        dest[index + 3] = block[n + 3] as u8;
        dest[index + 4] = block[n + 4] as u8;
        dest[index + 5] = block[n + 5] as u8;
        dest[index + 6] = block[n + 6] as u8;
        dest[index + 7] = block[n + 7] as u8;
        index += scan + 8;
    }
}

#[allow(clippy::identity_op)]
fn add_block_to_destination(block: &[i32; 64], dest: &mut [u8], mut index: usize, scan: usize) {
    for n in (0..64).step_by(8) {
        dest[index + 0] = (dest[index + 0] as i32 + block[n + 0]).clamp(0, 255) as u8;
        dest[index + 1] = (dest[index + 1] as i32 + block[n + 1]).clamp(0, 255) as u8;
        dest[index + 2] = (dest[index + 2] as i32 + block[n + 2]).clamp(0, 255) as u8;
        dest[index + 3] = (dest[index + 3] as i32 + block[n + 3]).clamp(0, 255) as u8;
        dest[index + 4] = (dest[index + 4] as i32 + block[n + 4]).clamp(0, 255) as u8;
        dest[index + 5] = (dest[index + 5] as i32 + block[n + 5]).clamp(0, 255) as u8;
        dest[index + 6] = (dest[index + 6] as i32 + block[n + 6]).clamp(0, 255) as u8;
        dest[index + 7] = (dest[index + 7] as i32 + block[n + 7]).clamp(0, 255) as u8;
        index += scan + 8;
    }
}

#[allow(clippy::identity_op)]
fn copy_value_to_destination(value: u8, dest: &mut [u8], mut index: usize, scan: usize) {
    for _ in (0..64).step_by(8) {
        dest[index + 0] = value;
        dest[index + 1] = value;
        dest[index + 2] = value;
        dest[index + 3] = value;
        dest[index + 4] = value;
        dest[index + 5] = value;
        dest[index + 6] = value;
        dest[index + 7] = value;
        index += scan + 8;
    }
}

#[allow(clippy::identity_op)]
fn add_value_to_destination(value: i32, dest: &mut [u8], mut index: usize, scan: usize) {
    for _ in (0..64).step_by(8) {
        dest[index + 0] = (dest[index + 0] as i32 + value).clamp(0, 255) as u8;
        dest[index + 1] = (dest[index + 1] as i32 + value).clamp(0, 255) as u8;
        dest[index + 2] = (dest[index + 2] as i32 + value).clamp(0, 255) as u8;
        dest[index + 3] = (dest[index + 3] as i32 + value).clamp(0, 255) as u8;
        dest[index + 4] = (dest[index + 4] as i32 + value).clamp(0, 255) as u8;
        dest[index + 5] = (dest[index + 5] as i32 + value).clamp(0, 255) as u8;
        dest[index + 6] = (dest[index + 6] as i32 + value).clamp(0, 255) as u8;
        dest[index + 7] = (dest[index + 7] as i32 + value).clamp(0, 255) as u8;
        index += scan + 8;
    }
}

#[rustfmt::skip]
#[allow(non_snake_case, clippy::identity_op, clippy::erasing_op)]
fn IDCT(block: &mut [i32; 64]) {
    let (mut b1, mut b3, mut b4, mut b6, mut b7, mut tmp1, mut tmp2, mut m0,
    mut x0, mut x1, mut x2, mut x3, mut x4, mut y3, mut y4, mut y5, mut y6, mut y7);

    // Transform columns
    for i in 0..8 {
        b1 = block[4*8+i];
        b3 = block[2*8+i] + block[6*8+i];
        b4 = block[5*8+i] - block[3*8+i];
        tmp1 = block[1*8+i] + block[7*8+i];
        tmp2 = block[3*8+i] + block[5*8+i];
        b6 = block[1*8+i] - block[7*8+i];
        b7 = tmp1 + tmp2;
        m0 = block[0*8+i];
        x4 = ((b6*473 - b4*196 + 128) >> 8) - b7;
        x0 = x4 - (((tmp1 - tmp2)*362 + 128) >> 8);
        x1 = m0 - b1;
        x2 = (((block[2*8+i] - block[6*8+i])*362 + 128) >> 8) - b3;
        x3 = m0 + b1;
        y3 = x1 + x2;
        y4 = x3 + b3;
        y5 = x1 - x2;
        y6 = x3 - b3;
        y7 = -x0 - ((b4*473 + b6*196 + 128) >> 8);
        block[0*8+i] = b7 + y4;
        block[1*8+i] = x4 + y3;
        block[2*8+i] = y5 - x0;
        block[3*8+i] = y6 - y7;
        block[4*8+i] = y6 + y7;
        block[5*8+i] = x0 + y5;
        block[6*8+i] = y3 - x4;
        block[7*8+i] = y4 - b7;
    }

    // Transform rows
    for i in (0..64).step_by(8) {
        b1 = block[4+i];
        b3 = block[2+i] + block[6+i];
        b4 = block[5+i] - block[3+i];
        tmp1 = block[1+i] + block[7+i];
        tmp2 = block[3+i] + block[5+i];
        b6 = block[1+i] - block[7+i];
        b7 = tmp1 + tmp2;
        m0 = block[0+i];
        x4 = ((b6*473 - b4*196 + 128) >> 8) - b7;
        x0 = x4 - (((tmp1 - tmp2)*362 + 128) >> 8);
        x1 = m0 - b1;
        x2 = (((block[2+i] - block[6+i])*362 + 128) >> 8) - b3;
        x3 = m0 + b1;
        y3 = x1 + x2;
        y4 = x3 + b3;
        y5 = x1 - x2;
        y6 = x3 - b3;
        y7 = -x0 - ((b4*473 + b6*196 + 128) >> 8);
        block[0+i] = (b7 + y4 + 128) >> 8;
        block[1+i] = (x4 + y3 + 128) >> 8;
        block[2+i] = (y5 - x0 + 128) >> 8;
        block[3+i] = (y6 - y7 + 128) >> 8;
        block[4+i] = (y6 + y7 + 128) >> 8;
        block[5+i] = (x0 + y5 + 128) >> 8;
        block[6+i] = (y3 - x4 + 128) >> 8;
        block[7+i] = (y4 - b7 + 128) >> 8;
    }
}

#[rustfmt::skip]
#[allow(clippy::identity_op)]
pub mod constants {
    pub const PICTURE_START_CODE: u32 = 0x00_00_01_00;
    pub const SLICE_FIRST_START_CODE: u32 = 0x00_00_01_01;
    pub const SLICE_LAST_START_CODE: u32 = 0x00_00_01_AF;
    pub const USER_DATA_START_CODE: u32 = 0x00_00_01_B2;
    pub const SEQUENCE_HEADER_CODE: u32 = 0x00_00_01_B3;
    pub const EXTENSION_START_CODE: u32 = 0x00_00_01_B5;

    pub const PICTURE_TYPE_INTRA: u8 = 0b001;
    pub const PICTURE_TYPE_PREDICTIVE: u8 = 0b010;
    pub const PICTURE_TYPE_B: u8 = 0b011;
    
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
    
    pub const MACROBLOCK_ADDRESS_INCREMENT: [i32; 75*3] = [
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
    
    pub const MACROBLOCK_TYPE_INTRA: [i32; 3*4] = [
        1*3,  2*3,     0, //   0
         -1,  3*3,     0, //   1  0
          0,    0,  0x01, //   2  1.
          0,    0,  0x11  //   3  01.
    ];

    pub const MACROBLOCK_TYPE_PREDICTIVE: [i32; 3*14]  = [
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


    pub const MACROBLOCK_TYPE_B: [i32; 3*23]  = [
        1*3,  2*3,     0, //  0
        3*3,  4*3,     0, //  1  0
        5*3,  6*3,     0, //  2  1
        7*3,  8*3,     0, //  3  00
        9*3, 10*3,     0, //  4  01
          0,    0,    12, //  5  10.
          0,    0,    14, //  6  11.
       11*3, 12*3,     0, //  7  000
       13*3, 14*3,     0, //  8  001
          0,    0,     4, //  9  010.
          0,    0,     6, // 10  011.
       15*3, 16*3,     0, // 11  0000
       17*3, 18*3,     0, // 12  0001
          0,    0,     8, // 13  0010.
          0,    0,    10, // 14  0011.
       19*3, 20*3,     0, // 15  0000 0
       21*3, 22*3,     0, // 16  0000 1
          0,    0,    30, // 17  0001 0.
          0,    0,     1, // 18  0001 1.
         -1,   -1,     0, // 19  0000 00
          0,    0,    17, // 20  0000 01.
          0,    0,    21, // 21  0000 10.
          0,    0,    26, // 22  0000 11.
    ];
        
    pub const CODE_BLOCK_PATTERN: [i32; 3*126] = [
        2*3,   1*3,   0,  //   0
        3*3,   6*3,   0,  //   1  1
        4*3,   5*3,   0,  //   2  0
        8*3,  11*3,   0,  //   3  10
       12*3,  13*3,   0,  //   4  00
        9*3,   7*3,   0,  //   5  01
       10*3,  14*3,   0,  //   6  11
       20*3,  19*3,   0,  //   7  011
       18*3,  16*3,   0,  //   8  100
       23*3,  17*3,   0,  //   9  010
       27*3,  25*3,   0,  //  10  110
       21*3,  28*3,   0,  //  11  101
       15*3,  22*3,   0,  //  12  000
       24*3,  26*3,   0,  //  13  001
          0,     0,  60,  //  14  111.
       35*3,  40*3,   0,  //  15  0000
       44*3,  48*3,   0,  //  16  1001
       38*3,  36*3,   0,  //  17  0101
       42*3,  47*3,   0,  //  18  1000
       29*3,  31*3,   0,  //  19  0111
       39*3,  32*3,   0,  //  20  0110
          0,     0,  32,  //  21  1010.
       45*3,  46*3,   0,  //  22  0001
       33*3,  41*3,   0,  //  23  0100
       43*3,  34*3,   0,  //  24  0010
          0,     0,   4,  //  25  1101.
       30*3,  37*3,   0,  //  26  0011
          0,     0,   8,  //  27  1100.
          0,     0,  16,  //  28  1011.
          0,     0,  44,  //  29  0111 0.
       50*3,  56*3,   0,  //  30  0011 0
          0,     0,  28,  //  31  0111 1.
          0,     0,  52,  //  32  0110 1.
          0,     0,  62,  //  33  0100 0.
       61*3,  59*3,   0,  //  34  0010 1
       52*3,  60*3,   0,  //  35  0000 0
          0,     0,   1,  //  36  0101 1.
       55*3,  54*3,   0,  //  37  0011 1
          0,     0,  61,  //  38  0101 0.
          0,     0,  56,  //  39  0110 0.
       57*3,  58*3,   0,  //  40  0000 1
          0,     0,   2,  //  41  0100 1.
          0,     0,  40,  //  42  1000 0.
       51*3,  62*3,   0,  //  43  0010 0
          0,     0,  48,  //  44  1001 0.
       64*3,  63*3,   0,  //  45  0001 0
       49*3,  53*3,   0,  //  46  0001 1
          0,     0,  20,  //  47  1000 1.
          0,     0,  12,  //  48  1001 1.
       80*3,  83*3,   0,  //  49  0001 10
          0,     0,  63,  //  50  0011 00.
       77*3,  75*3,   0,  //  51  0010 00
       65*3,  73*3,   0,  //  52  0000 00
       84*3,  66*3,   0,  //  53  0001 11
          0,     0,  24,  //  54  0011 11.
          0,     0,  36,  //  55  0011 10.
          0,     0,   3,  //  56  0011 01.
       69*3,  87*3,   0,  //  57  0000 10
       81*3,  79*3,   0,  //  58  0000 11
       68*3,  71*3,   0,  //  59  0010 11
       70*3,  78*3,   0,  //  60  0000 01
       67*3,  76*3,   0,  //  61  0010 10
       72*3,  74*3,   0,  //  62  0010 01
       86*3,  85*3,   0,  //  63  0001 01
       88*3,  82*3,   0,  //  64  0001 00
         -1,  94*3,   0,  //  65  0000 000
       95*3,  97*3,   0,  //  66  0001 111
          0,     0,  33,  //  67  0010 100.
          0,     0,   9,  //  68  0010 110.
      106*3, 110*3,   0,  //  69  0000 100
      102*3, 116*3,   0,  //  70  0000 010
          0,     0,   5,  //  71  0010 111.
          0,     0,  10,  //  72  0010 010.
       93*3,  89*3,   0,  //  73  0000 001
          0,     0,   6,  //  74  0010 011.
          0,     0,  18,  //  75  0010 001.
          0,     0,  17,  //  76  0010 101.
          0,     0,  34,  //  77  0010 000.
      113*3, 119*3,   0,  //  78  0000 011
      103*3, 104*3,   0,  //  79  0000 111
       90*3,  92*3,   0,  //  80  0001 100
      109*3, 107*3,   0,  //  81  0000 110
      117*3, 118*3,   0,  //  82  0001 001
      101*3,  99*3,   0,  //  83  0001 101
       98*3,  96*3,   0,  //  84  0001 110
      100*3,  91*3,   0,  //  85  0001 011
      114*3, 115*3,   0,  //  86  0001 010
      105*3, 108*3,   0,  //  87  0000 101
      112*3, 111*3,   0,  //  88  0001 000
      121*3, 125*3,   0,  //  89  0000 0011
          0,     0,  41,  //  90  0001 1000.
          0,     0,  14,  //  91  0001 0111.
          0,     0,  21,  //  92  0001 1001.
      124*3, 122*3,   0,  //  93  0000 0010
      120*3, 123*3,   0,  //  94  0000 0001
          0,     0,  11,  //  95  0001 1110.
          0,     0,  19,  //  96  0001 1101.
          0,     0,   7,  //  97  0001 1111.
          0,     0,  35,  //  98  0001 1100.
          0,     0,  13,  //  99  0001 1011.
          0,     0,  50,  // 100  0001 0110.
          0,     0,  49,  // 101  0001 1010.
          0,     0,  58,  // 102  0000 0100.
          0,     0,  37,  // 103  0000 1110.
          0,     0,  25,  // 104  0000 1111.
          0,     0,  45,  // 105  0000 1010.
          0,     0,  57,  // 106  0000 1000.
          0,     0,  26,  // 107  0000 1101.
          0,     0,  29,  // 108  0000 1011.
          0,     0,  38,  // 109  0000 1100.
          0,     0,  53,  // 110  0000 1001.
          0,     0,  23,  // 111  0001 0001.
          0,     0,  43,  // 112  0001 0000.
          0,     0,  46,  // 113  0000 0110.
          0,     0,  42,  // 114  0001 0100.
          0,     0,  22,  // 115  0001 0101.
          0,     0,  54,  // 116  0000 0101.
          0,     0,  51,  // 117  0001 0010.
          0,     0,  15,  // 118  0001 0011.
          0,     0,  30,  // 119  0000 0111.
          0,     0,  39,  // 120  0000 0001 0.
          0,     0,  47,  // 121  0000 0011 0.
          0,     0,  55,  // 122  0000 0010 1.
          0,     0,  27,  // 123  0000 0001 1.
          0,     0,  59,  // 124  0000 0010 0.
          0,     0,  31   // 125  0000 0011 1.
    ];
    
    pub const DCT_DC_SIZE_LUMINANCE: [i32; 3*18] = [
        2*3,   1*3, 0,  //   0
        6*3,   5*3, 0,  //   1  1
        3*3,   4*3, 0,  //   2  0
          0,     0, 1,  //   3  00.
          0,     0, 2,  //   4  01.
        9*3,   8*3, 0,  //   5  11
        7*3,  10*3, 0,  //   6  10
          0,     0, 0,  //   7  100.
       12*3,  11*3, 0,  //   8  111
          0,     0, 4,  //   9  110.
          0,     0, 3,  //  10  101.
       13*3,  14*3, 0,  //  11  1111
          0,     0, 5,  //  12  1110.
          0,     0, 6,  //  13  1111 0.
       16*3,  15*3, 0,  //  14  1111 1
       17*3,    -1, 0,  //  15  1111 11
          0,     0, 7,  //  16  1111 10.
          0,     0, 8   //  17  1111 110.
    ];
    
    pub const DCT_DC_SIZE_CHROMINANCE: [i32; 3*18] = [
        2*3,   1*3, 0,  //   0
        4*3,   3*3, 0,  //   1  1
        6*3,   5*3, 0,  //   2  0
        8*3,   7*3, 0,  //   3  11
          0,     0, 2,  //   4  10.
          0,     0, 1,  //   5  01.
          0,     0, 0,  //   6  00.
       10*3,   9*3, 0,  //   7  111
          0,     0, 3,  //   8  110.
       12*3,  11*3, 0,  //   9  1111
          0,     0, 4,  //  10  1110.
       14*3,  13*3, 0,  //  11  1111 1
          0,     0, 5,  //  12  1111 0.
       16*3,  15*3, 0,  //  13  1111 11
          0,     0, 6,  //  14  1111 10.
       17*3,    -1, 0,  //  15  1111 111
          0,     0, 7,  //  16  1111 110.
          0,     0, 8   //  17  1111 1110.
    ];
    pub const MOTION: [i32; 3*67] = [
        1*3,   2*3,   0,  //   0
        4*3,   3*3,   0,  //   1  0
          0,     0,   0,  //   2  1.
        6*3,   5*3,   0,  //   3  01
        8*3,   7*3,   0,  //   4  00
          0,     0,  -1,  //   5  011.
          0,     0,   1,  //   6  010.
        9*3,  10*3,   0,  //   7  001
       12*3,  11*3,   0,  //   8  000
          0,     0,   2,  //   9  0010.
          0,     0,  -2,  //  10  0011.
       14*3,  15*3,   0,  //  11  0001
       16*3,  13*3,   0,  //  12  0000
       20*3,  18*3,   0,  //  13  0000 1
          0,     0,   3,  //  14  0001 0.
          0,     0,  -3,  //  15  0001 1.
       17*3,  19*3,   0,  //  16  0000 0
         -1,  23*3,   0,  //  17  0000 00
       27*3,  25*3,   0,  //  18  0000 11
       26*3,  21*3,   0,  //  19  0000 01
       24*3,  22*3,   0,  //  20  0000 10
       32*3,  28*3,   0,  //  21  0000 011
       29*3,  31*3,   0,  //  22  0000 101
         -1,  33*3,   0,  //  23  0000 001
       36*3,  35*3,   0,  //  24  0000 100
          0,     0,  -4,  //  25  0000 111.
       30*3,  34*3,   0,  //  26  0000 010
          0,     0,   4,  //  27  0000 110.
          0,     0,  -7,  //  28  0000 0111.
          0,     0,   5,  //  29  0000 1010.
       37*3,  41*3,   0,  //  30  0000 0100
          0,     0,  -5,  //  31  0000 1011.
          0,     0,   7,  //  32  0000 0110.
       38*3,  40*3,   0,  //  33  0000 0011
       42*3,  39*3,   0,  //  34  0000 0101
          0,     0,  -6,  //  35  0000 1001.
          0,     0,   6,  //  36  0000 1000.
       51*3,  54*3,   0,  //  37  0000 0100 0
       50*3,  49*3,   0,  //  38  0000 0011 0
       45*3,  46*3,   0,  //  39  0000 0101 1
       52*3,  47*3,   0,  //  40  0000 0011 1
       43*3,  53*3,   0,  //  41  0000 0100 1
       44*3,  48*3,   0,  //  42  0000 0101 0
          0,     0,  10,  //  43  0000 0100 10.
          0,     0,   9,  //  44  0000 0101 00.
          0,     0,   8,  //  45  0000 0101 10.
          0,     0,  -8,  //  46  0000 0101 11.
       57*3,  66*3,   0,  //  47  0000 0011 11
          0,     0,  -9,  //  48  0000 0101 01.
       60*3,  64*3,   0,  //  49  0000 0011 01
       56*3,  61*3,   0,  //  50  0000 0011 00
       55*3,  62*3,   0,  //  51  0000 0100 00
       58*3,  63*3,   0,  //  52  0000 0011 10
          0,     0, -10,  //  53  0000 0100 11.
       59*3,  65*3,   0,  //  54  0000 0100 01
          0,     0,  12,  //  55  0000 0100 000.
          0,     0,  16,  //  56  0000 0011 000.
          0,     0,  13,  //  57  0000 0011 110.
          0,     0,  14,  //  58  0000 0011 100.
          0,     0,  11,  //  59  0000 0100 010.
          0,     0,  15,  //  60  0000 0011 010.
          0,     0, -16,  //  61  0000 0011 001.
          0,     0, -12,  //  62  0000 0100 001.
          0,     0, -14,  //  63  0000 0011 101.
          0,     0, -15,  //  64  0000 0011 011.
          0,     0, -11,  //  65  0000 0100 011.
          0,     0, -13   //  66  0000 0011 111.
    ];              
    
    pub const DCT_COEFF: [i32; 3*224] = [
        1*3,   2*3,      0,  //   0
        4*3,   3*3,      0,  //   1  0
          0,     0, 0x0001,  //   2  1.
        7*3,   8*3,      0,  //   3  01
        6*3,   5*3,      0,  //   4  00
       13*3,   9*3,      0,  //   5  001
       11*3,  10*3,      0,  //   6  000
       14*3,  12*3,      0,  //   7  010
          0,     0, 0x0101,  //   8  011.
       20*3,  22*3,      0,  //   9  0011
       18*3,  21*3,      0,  //  10  0001
       16*3,  19*3,      0,  //  11  0000
          0,     0, 0x0201,  //  12  0101.
       17*3,  15*3,      0,  //  13  0010
          0,     0, 0x0002,  //  14  0100.
          0,     0, 0x0003,  //  15  0010 1.
       27*3,  25*3,      0,  //  16  0000 0
       29*3,  31*3,      0,  //  17  0010 0
       24*3,  26*3,      0,  //  18  0001 0
       32*3,  30*3,      0,  //  19  0000 1
          0,     0, 0x0401,  //  20  0011 0.
       23*3,  28*3,      0,  //  21  0001 1
          0,     0, 0x0301,  //  22  0011 1.
          0,     0, 0x0102,  //  23  0001 10.
          0,     0, 0x0701,  //  24  0001 00.
          0,     0, 0xffff,  //  25  0000 01. -- escape
          0,     0, 0x0601,  //  26  0001 01.
       37*3,  36*3,      0,  //  27  0000 00
          0,     0, 0x0501,  //  28  0001 11.
       35*3,  34*3,      0,  //  29  0010 00
       39*3,  38*3,      0,  //  30  0000 11
       33*3,  42*3,      0,  //  31  0010 01
       40*3,  41*3,      0,  //  32  0000 10
       52*3,  50*3,      0,  //  33  0010 010
       54*3,  53*3,      0,  //  34  0010 001
       48*3,  49*3,      0,  //  35  0010 000
       43*3,  45*3,      0,  //  36  0000 001
       46*3,  44*3,      0,  //  37  0000 000
          0,     0, 0x0801,  //  38  0000 111.
          0,     0, 0x0004,  //  39  0000 110.
          0,     0, 0x0202,  //  40  0000 100.
          0,     0, 0x0901,  //  41  0000 101.
       51*3,  47*3,      0,  //  42  0010 011
       55*3,  57*3,      0,  //  43  0000 0010
       60*3,  56*3,      0,  //  44  0000 0001
       59*3,  58*3,      0,  //  45  0000 0011
       61*3,  62*3,      0,  //  46  0000 0000
          0,     0, 0x0a01,  //  47  0010 0111.
          0,     0, 0x0d01,  //  48  0010 0000.
          0,     0, 0x0006,  //  49  0010 0001.
          0,     0, 0x0103,  //  50  0010 0101.
          0,     0, 0x0005,  //  51  0010 0110.
          0,     0, 0x0302,  //  52  0010 0100.
          0,     0, 0x0b01,  //  53  0010 0011.
          0,     0, 0x0c01,  //  54  0010 0010.
       76*3,  75*3,      0,  //  55  0000 0010 0
       67*3,  70*3,      0,  //  56  0000 0001 1
       73*3,  71*3,      0,  //  57  0000 0010 1
       78*3,  74*3,      0,  //  58  0000 0011 1
       72*3,  77*3,      0,  //  59  0000 0011 0
       69*3,  64*3,      0,  //  60  0000 0001 0
       68*3,  63*3,      0,  //  61  0000 0000 0
       66*3,  65*3,      0,  //  62  0000 0000 1
       81*3,  87*3,      0,  //  63  0000 0000 01
       91*3,  80*3,      0,  //  64  0000 0001 01
       82*3,  79*3,      0,  //  65  0000 0000 11
       83*3,  86*3,      0,  //  66  0000 0000 10
       93*3,  92*3,      0,  //  67  0000 0001 10
       84*3,  85*3,      0,  //  68  0000 0000 00
       90*3,  94*3,      0,  //  69  0000 0001 00
       88*3,  89*3,      0,  //  70  0000 0001 11
          0,     0, 0x0203,  //  71  0000 0010 11.
          0,     0, 0x0104,  //  72  0000 0011 00.
          0,     0, 0x0007,  //  73  0000 0010 10.
          0,     0, 0x0402,  //  74  0000 0011 11.
          0,     0, 0x0502,  //  75  0000 0010 01.
          0,     0, 0x1001,  //  76  0000 0010 00.
          0,     0, 0x0f01,  //  77  0000 0011 01.
          0,     0, 0x0e01,  //  78  0000 0011 10.
      105*3, 107*3,      0,  //  79  0000 0000 111
      111*3, 114*3,      0,  //  80  0000 0001 011
      104*3,  97*3,      0,  //  81  0000 0000 010
      125*3, 119*3,      0,  //  82  0000 0000 110
       96*3,  98*3,      0,  //  83  0000 0000 100
         -1, 123*3,      0,  //  84  0000 0000 000
       95*3, 101*3,      0,  //  85  0000 0000 001
      106*3, 121*3,      0,  //  86  0000 0000 101
       99*3, 102*3,      0,  //  87  0000 0000 011
      113*3, 103*3,      0,  //  88  0000 0001 110
      112*3, 116*3,      0,  //  89  0000 0001 111
      110*3, 100*3,      0,  //  90  0000 0001 000
      124*3, 115*3,      0,  //  91  0000 0001 010
      117*3, 122*3,      0,  //  92  0000 0001 101
      109*3, 118*3,      0,  //  93  0000 0001 100
      120*3, 108*3,      0,  //  94  0000 0001 001
      127*3, 136*3,      0,  //  95  0000 0000 0010
      139*3, 140*3,      0,  //  96  0000 0000 1000
      130*3, 126*3,      0,  //  97  0000 0000 0101
      145*3, 146*3,      0,  //  98  0000 0000 1001
      128*3, 129*3,      0,  //  99  0000 0000 0110
          0,     0, 0x0802,  // 100  0000 0001 0001.
      132*3, 134*3,      0,  // 101  0000 0000 0011
      155*3, 154*3,      0,  // 102  0000 0000 0111
          0,     0, 0x0008,  // 103  0000 0001 1101.
      137*3, 133*3,      0,  // 104  0000 0000 0100
      143*3, 144*3,      0,  // 105  0000 0000 1110
      151*3, 138*3,      0,  // 106  0000 0000 1010
      142*3, 141*3,      0,  // 107  0000 0000 1111
          0,     0, 0x000a,  // 108  0000 0001 0011.
          0,     0, 0x0009,  // 109  0000 0001 1000.
          0,     0, 0x000b,  // 110  0000 0001 0000.
          0,     0, 0x1501,  // 111  0000 0001 0110.
          0,     0, 0x0602,  // 112  0000 0001 1110.
          0,     0, 0x0303,  // 113  0000 0001 1100.
          0,     0, 0x1401,  // 114  0000 0001 0111.
          0,     0, 0x0702,  // 115  0000 0001 0101.
          0,     0, 0x1101,  // 116  0000 0001 1111.
          0,     0, 0x1201,  // 117  0000 0001 1010.
          0,     0, 0x1301,  // 118  0000 0001 1001.
      148*3, 152*3,      0,  // 119  0000 0000 1101
          0,     0, 0x0403,  // 120  0000 0001 0010.
      153*3, 150*3,      0,  // 121  0000 0000 1011
          0,     0, 0x0105,  // 122  0000 0001 1011.
      131*3, 135*3,      0,  // 123  0000 0000 0001
          0,     0, 0x0204,  // 124  0000 0001 0100.
      149*3, 147*3,      0,  // 125  0000 0000 1100
      172*3, 173*3,      0,  // 126  0000 0000 0101 1
      162*3, 158*3,      0,  // 127  0000 0000 0010 0
      170*3, 161*3,      0,  // 128  0000 0000 0110 0
      168*3, 166*3,      0,  // 129  0000 0000 0110 1
      157*3, 179*3,      0,  // 130  0000 0000 0101 0
      169*3, 167*3,      0,  // 131  0000 0000 0001 0
      174*3, 171*3,      0,  // 132  0000 0000 0011 0
      178*3, 177*3,      0,  // 133  0000 0000 0100 1
      156*3, 159*3,      0,  // 134  0000 0000 0011 1
      164*3, 165*3,      0,  // 135  0000 0000 0001 1
      183*3, 182*3,      0,  // 136  0000 0000 0010 1
      175*3, 176*3,      0,  // 137  0000 0000 0100 0
          0,     0, 0x0107,  // 138  0000 0000 1010 1.
          0,     0, 0x0a02,  // 139  0000 0000 1000 0.
          0,     0, 0x0902,  // 140  0000 0000 1000 1.
          0,     0, 0x1601,  // 141  0000 0000 1111 1.
          0,     0, 0x1701,  // 142  0000 0000 1111 0.
          0,     0, 0x1901,  // 143  0000 0000 1110 0.
          0,     0, 0x1801,  // 144  0000 0000 1110 1.
          0,     0, 0x0503,  // 145  0000 0000 1001 0.
          0,     0, 0x0304,  // 146  0000 0000 1001 1.
          0,     0, 0x000d,  // 147  0000 0000 1100 1.
          0,     0, 0x000c,  // 148  0000 0000 1101 0.
          0,     0, 0x000e,  // 149  0000 0000 1100 0.
          0,     0, 0x000f,  // 150  0000 0000 1011 1.
          0,     0, 0x0205,  // 151  0000 0000 1010 0.
          0,     0, 0x1a01,  // 152  0000 0000 1101 1.
          0,     0, 0x0106,  // 153  0000 0000 1011 0.
      180*3, 181*3,      0,  // 154  0000 0000 0111 1
      160*3, 163*3,      0,  // 155  0000 0000 0111 0
      196*3, 199*3,      0,  // 156  0000 0000 0011 10
          0,     0, 0x001b,  // 157  0000 0000 0101 00.
      203*3, 185*3,      0,  // 158  0000 0000 0010 01
      202*3, 201*3,      0,  // 159  0000 0000 0011 11
          0,     0, 0x0013,  // 160  0000 0000 0111 00.
          0,     0, 0x0016,  // 161  0000 0000 0110 01.
      197*3, 207*3,      0,  // 162  0000 0000 0010 00
          0,     0, 0x0012,  // 163  0000 0000 0111 01.
      191*3, 192*3,      0,  // 164  0000 0000 0001 10
      188*3, 190*3,      0,  // 165  0000 0000 0001 11
          0,     0, 0x0014,  // 166  0000 0000 0110 11.
      184*3, 194*3,      0,  // 167  0000 0000 0001 01
          0,     0, 0x0015,  // 168  0000 0000 0110 10.
      186*3, 193*3,      0,  // 169  0000 0000 0001 00
          0,     0, 0x0017,  // 170  0000 0000 0110 00.
      204*3, 198*3,      0,  // 171  0000 0000 0011 01
          0,     0, 0x0019,  // 172  0000 0000 0101 10.
          0,     0, 0x0018,  // 173  0000 0000 0101 11.
      200*3, 205*3,      0,  // 174  0000 0000 0011 00
          0,     0, 0x001f,  // 175  0000 0000 0100 00.
          0,     0, 0x001e,  // 176  0000 0000 0100 01.
          0,     0, 0x001c,  // 177  0000 0000 0100 11.
          0,     0, 0x001d,  // 178  0000 0000 0100 10.
          0,     0, 0x001a,  // 179  0000 0000 0101 01.
          0,     0, 0x0011,  // 180  0000 0000 0111 10.
          0,     0, 0x0010,  // 181  0000 0000 0111 11.
      189*3, 206*3,      0,  // 182  0000 0000 0010 11
      187*3, 195*3,      0,  // 183  0000 0000 0010 10
      218*3, 211*3,      0,  // 184  0000 0000 0001 010
          0,     0, 0x0025,  // 185  0000 0000 0010 011.
      215*3, 216*3,      0,  // 186  0000 0000 0001 000
          0,     0, 0x0024,  // 187  0000 0000 0010 100.
      210*3, 212*3,      0,  // 188  0000 0000 0001 110
          0,     0, 0x0022,  // 189  0000 0000 0010 110.
      213*3, 209*3,      0,  // 190  0000 0000 0001 111
      221*3, 222*3,      0,  // 191  0000 0000 0001 100
      219*3, 208*3,      0,  // 192  0000 0000 0001 101
      217*3, 214*3,      0,  // 193  0000 0000 0001 001
      223*3, 220*3,      0,  // 194  0000 0000 0001 011
          0,     0, 0x0023,  // 195  0000 0000 0010 101.
          0,     0, 0x010b,  // 196  0000 0000 0011 100.
          0,     0, 0x0028,  // 197  0000 0000 0010 000.
          0,     0, 0x010c,  // 198  0000 0000 0011 011.
          0,     0, 0x010a,  // 199  0000 0000 0011 101.
          0,     0, 0x0020,  // 200  0000 0000 0011 000.
          0,     0, 0x0108,  // 201  0000 0000 0011 111.
          0,     0, 0x0109,  // 202  0000 0000 0011 110.
          0,     0, 0x0026,  // 203  0000 0000 0010 010.
          0,     0, 0x010d,  // 204  0000 0000 0011 010.
          0,     0, 0x010e,  // 205  0000 0000 0011 001.
          0,     0, 0x0021,  // 206  0000 0000 0010 111.
          0,     0, 0x0027,  // 207  0000 0000 0010 001.
          0,     0, 0x1f01,  // 208  0000 0000 0001 1011.
          0,     0, 0x1b01,  // 209  0000 0000 0001 1111.
          0,     0, 0x1e01,  // 210  0000 0000 0001 1100.
          0,     0, 0x1002,  // 211  0000 0000 0001 0101.
          0,     0, 0x1d01,  // 212  0000 0000 0001 1101.
          0,     0, 0x1c01,  // 213  0000 0000 0001 1110.
          0,     0, 0x010f,  // 214  0000 0000 0001 0011.
          0,     0, 0x0112,  // 215  0000 0000 0001 0000.
          0,     0, 0x0111,  // 216  0000 0000 0001 0001.
          0,     0, 0x0110,  // 217  0000 0000 0001 0010.
          0,     0, 0x0603,  // 218  0000 0000 0001 0100.
          0,     0, 0x0b02,  // 219  0000 0000 0001 1010.
          0,     0, 0x0e02,  // 220  0000 0000 0001 0111.
          0,     0, 0x0d02,  // 221  0000 0000 0001 1000.
          0,     0, 0x0c02,  // 222  0000 0000 0001 1001.
          0,     0, 0x0f02   // 223  0000 0000 0001 0110.
  ];
    
  
    pub const PREMULTIPLIER_MATRIX: [u8; 64] = [
        32, 44, 42, 38, 32, 25, 17,  9,
        44, 62, 58, 52, 44, 35, 24, 12,
        42, 58, 55, 49, 42, 33, 23, 12,
        38, 52, 49, 44, 38, 30, 20, 10,
        32, 44, 42, 38, 32, 25, 17,  9,
        25, 35, 33, 30, 25, 20, 14,  7,
        17, 24, 23, 20, 17, 14,  9,  5,
         9, 12, 12, 10,  9,  7,  5,  2
    ];
}

#[cfg(test)]
mod test {
    use crate::section::mpeg_visualization::{mpeg1::MPEG1, ts::TSDemuxer};

    #[test]
    fn test() {
        let bytes = include_bytes!("test.ts");
        let demuxed_bytes = TSDemuxer::from_raw_bytes(bytes.to_vec()).parse_packets();
        let mut mpeg1 = MPEG1::from_bytes(demuxed_bytes);

        for _ in 0..30 {
            let test = mpeg1.decode().unwrap();
            let test2 = mpeg1.decode().unwrap();
        }
    }
}