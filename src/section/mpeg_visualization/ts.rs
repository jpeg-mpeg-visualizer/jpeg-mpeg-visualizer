use std::collections::HashMap;

use bitvec::prelude::*;

pub struct TSDemuxer {
    buffer: BitVec<Msb0, u8>,
    pointer: usize,
    pids_to_stream_id: HashMap<u16, u8>,
    video_packet_buffer: Vec<u8>,
}

impl TSDemuxer {
    pub fn from_raw_bytes(raw_bytes: Vec<u8>) -> TSDemuxer {
        TSDemuxer {
            buffer: BitVec::<Msb0, _>::from_vec(raw_bytes),
            pointer: 0,
            pids_to_stream_id: HashMap::new(),
            video_packet_buffer: Vec::new(),
        }
    }

    pub fn parse_packets(&mut self) -> Vec<u8> {
        while self.buffer.len() - self.pointer >= 188 {
            self.parse_packet();
        }
        std::mem::take(&mut self.video_packet_buffer)
    }

    pub fn parse_packet(&mut self) {
        let end_pointer = self.pointer + 188 * 8;

        // Read the Sync byte
        let sync_byte = self.buffer[self.pointer..self.pointer + 8].load_be::<u8>();
        assert!(sync_byte == 0x47);
        self.pointer += 8;

        // Read the rest of the header
        let _transport_error_indicator =
            self.buffer[self.pointer..self.pointer + 1].load_be::<u8>();
        let payload_start = self.buffer[self.pointer + 1..self.pointer + 2].load_be::<u8>();
        let _transport_priority = self.buffer[self.pointer + 2..self.pointer + 3].load_be::<u8>();
        let pid = self.buffer[self.pointer + 3..self.pointer + 3 + 13].load_be::<u16>();
        let _transport_scrambling =
            self.buffer[self.pointer + 16..self.pointer + 16 + 2].load_be::<u8>();
        let adaptation_field_control =
            self.buffer[self.pointer + 18..self.pointer + 18 + 2].load_be::<u8>();
        let _continuity_counter = self.buffer[self.pointer + 20..self.pointer + 24].load_be::<u8>();
        self.pointer += 24;

        let mut stream_id = self.pids_to_stream_id.get(&pid).copied();

        // Extract payload if present
        if adaptation_field_control & 0b01 == 1 {
            // adaptation field is present, skip over it
            if adaptation_field_control & 0b10 == 0b10 {
                let adaptation_field_length =
                    self.buffer[self.pointer..self.pointer + 8].load_be::<u8>() as usize;
                self.pointer += 8 + adaptation_field_length * 8;
            }

            // The beginning of a new PES
            if payload_start == 1
                && self.buffer[self.pointer..self.pointer + 24].load_be::<u32>() == 1
            {
                self.pointer += 24;

                stream_id = Some(self.buffer[self.pointer..self.pointer + 8].load_be::<u8>());
                self.pids_to_stream_id.insert(pid, stream_id.unwrap());

                // Skip over 24 bits of data:
                // Packet length - 16 bits
                // Marker bits - 2 bits
                // Scrambling control - 2 bits
                // Priority - 1 bit
                // Data alignment indicator - 1 bit
                // Copyright - 1 bit
                // Original or Copy - 1 bit
                self.pointer += 32;

                // let pts_dts_indicator = self.buffer[self.pointer .. self.pointer+2].load_be::<u8>();
                // Skip over 6 bits of data:
                // ESCR flag - 1 bits
                // ES rate flag - 1 bits
                // DSM trick mode flag - 1 bit
                // Additional copy info flag - 1 bit
                // CRC flag - 1 bit
                // extension flag - 1 bit
                self.pointer += 8;

                let header_length =
                    self.buffer[self.pointer..self.pointer + 8].load_be::<u8>() as usize;

                // Skip over whole header (TODO we might want to revisit presentation timestamp later on)
                self.pointer += 8 + header_length * 8;
            }

            // we are currently reading video stream
            if let Some(0xE0) = stream_id {
                self.video_packet_buffer.extend(
                    self.buffer[self.pointer..end_pointer]
                        .to_bitvec()
                        .as_raw_slice(),
                );
            }

            self.pointer = end_pointer;
        }
    }
}
