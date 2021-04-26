use bitvec_helpers::bitvec_reader::BitVecReader;
use nom::{bytes::complete::take_until, IResult};

pub mod hevc;
pub mod utils;

use hevc::*;
use pps::PPSNAL;
use slice::SliceNAL;
use sps::SPSNAL;
use vps::VPSNAL;

use utils::clear_start_code_emulation_prevention_3_byte;

// We don't want to parse large slices because the memory is copied
const MAX_PARSE_SIZE: usize = 2048;

const HEADER_LEN: usize = 3;
const NAL_START_CODE: &[u8] = &[0, 0, 1];

#[derive(Default)]
pub struct HevcParser {
    reader: BitVecReader,

    nals: Vec<NALUnit>,
    vps: Vec<VPSNAL>,
    sps: Vec<SPSNAL>,
    pps: Vec<PPSNAL>,
    ordered_frames: Vec<Frame>,
    frames: Vec<Frame>,

    poc: u64,
    poc_tid0: u64,

    current_frame: Frame,
    decoded_index: u64,
    presentation_index: u64,
}

impl HevcParser {
    fn take_until_nal(data: &[u8]) -> IResult<&[u8], &[u8]> {
        take_until(NAL_START_CODE)(data)
    }

    pub fn get_offsets(&mut self, data: &[u8], offsets: &mut Vec<usize>) {
        offsets.clear();

        let mut consumed = 0;

        loop {
            match Self::take_until_nal(&data[consumed..]) {
                Ok(nal) => {
                    // Byte count before the NAL is the offset
                    consumed += nal.1.len();

                    offsets.push(consumed);

                    // nom consumes the tag, so add it back
                    consumed += HEADER_LEN;
                }
                _ => return,
            }
        }
    }

    pub fn split_nals(
        &mut self,
        data: &[u8],
        offsets: &[usize],
        last: usize,
        parse_nals: bool,
    ) -> Vec<NALUnit> {
        let count = offsets.len();

        let mut nals = Vec::with_capacity(count);

        for (index, offset) in offsets.iter().enumerate() {
            let size = if offset == &last {
                data.len() - offset
            } else {
                let size = if index == count - 1 {
                    last - offset
                } else {
                    offsets[index + 1] - offset
                };

                match &data[offset + size - 1..offset + size + 3] {
                    [0, 0, 0, 1] => size - 1,
                    _ => size,
                }
            };

            let nal: NALUnit = self.parse_nal(data, *offset, size, parse_nals);

            nals.push(nal);
        }

        nals
    }

    fn parse_nal(&mut self, data: &[u8], offset: usize, size: usize, parse_nal: bool) -> NALUnit {
        let mut nal = NALUnit::default();

        // Assuming [0, 0, 0, 1] header
        // Offset is at first element
        let pos = offset + 3;
        let end = offset + size;

        let parsing_end = if size > MAX_PARSE_SIZE {
            offset + MAX_PARSE_SIZE
        } else {
            end
        };

        nal.start = pos;
        nal.end = end;
        nal.decoded_frame_index = self.decoded_index;

        if parse_nal {
            let bytes = clear_start_code_emulation_prevention_3_byte(&data[pos..parsing_end]);
            self.reader = BitVecReader::new(bytes);

            self.parse_nal_header(&mut nal);

            self.nals.push(nal.clone());
        } else {
            nal.nal_type = data[pos] >> 1;
        }

        if nal.nuh_layer_id > 0 {
            return nal;
        }

        if parse_nal {
            match nal.nal_type {
                NAL_VPS => self.parse_vps(),
                NAL_SPS => self.parse_sps(),
                NAL_PPS => self.parse_pps(),

                NAL_TRAIL_R | NAL_TRAIL_N | NAL_TSA_N | NAL_TSA_R | NAL_STSA_N | NAL_STSA_R
                | NAL_BLA_W_LP | NAL_BLA_W_RADL | NAL_BLA_N_LP | NAL_IDR_W_RADL | NAL_IDR_N_LP
                | NAL_CRA_NUT | NAL_RADL_N | NAL_RADL_R | NAL_RASL_N | NAL_RASL_R => {
                    self.parse_slice(&mut nal);

                    self.current_frame.nals.push(nal.clone());
                }
                NAL_SEI_SUFFIX | NAL_UNSPEC62 | NAL_UNSPEC63 => {
                    // Dolby NALs are suffixed to the slices
                    self.current_frame.nals.push(nal.clone());
                }
                _ => {
                    self.add_current_frame();

                    nal.decoded_frame_index = self.decoded_index;
                    self.current_frame.nals.push(nal.clone());
                }
            };
        }

        nal
    }

    fn parse_nal_header(&mut self, nal: &mut NALUnit) {
        // forbidden_zero_bit
        self.reader.get();

        nal.nal_type = self.reader.get_n(6);
        nal.nuh_layer_id = self.reader.get_n(6);
        nal.temporal_id = self.reader.get_n::<u8>(3) - 1;
    }

    fn parse_vps(&mut self) {
        let vps = VPSNAL::parse(&mut self.reader);

        self.remove_vps(&vps);

        self.vps.push(vps);
    }

    fn parse_sps(&mut self) {
        let sps = SPSNAL::parse(&mut self.reader);
        self.remove_sps(&sps);

        self.sps.push(sps);
    }

    fn parse_pps(&mut self) {
        let pps = PPSNAL::parse(&mut self.reader, &self.sps);

        self.remove_pps(&pps);

        self.pps.push(pps);
    }

    fn parse_slice(&mut self, nal: &mut NALUnit) {
        let slice = SliceNAL::parse(
            &mut self.reader,
            &self.sps,
            &self.pps,
            nal,
            &mut self.poc_tid0,
            &mut self.poc,
        );

        // Consecutive slice NALs cases
        if self.current_frame.first_slice.first_slice_in_pic_flag && slice.first_slice_in_pic_flag {
            nal.decoded_frame_index = self.decoded_index + 1;
            self.add_current_frame();
        }

        if slice.key_frame {
            self.reorder_frames();
        }

        if slice.first_slice_in_pic_flag {
            self.current_frame.first_slice = slice;

            self.current_frame.decoded_number = self.decoded_index;
        }
    }

    fn remove_vps(&mut self, vps: &VPSNAL) {
        let id = vps.vps_id as usize;

        if let Some(existing_vps) = self.vps.get(id) {
            if existing_vps == vps {
                self.vps.remove(id);

                let sps_to_remove: Vec<SPSNAL> = self
                    .sps
                    .clone()
                    .into_iter()
                    .filter(|sps| sps.vps_id == vps.vps_id)
                    .collect();

                sps_to_remove.iter().for_each(|sps| self.remove_sps(sps));
            }
        }
    }

    fn remove_sps(&mut self, sps: &SPSNAL) {
        let id = sps.sps_id as usize;

        if let Some(existing_sps) = self.sps.get(id) {
            if existing_sps == sps {
                self.sps.remove(id);

                // Remove all dependent pps
                self.pps.retain(|pps| pps.sps_id != sps.sps_id);
            }
        }
    }

    fn remove_pps(&mut self, pps: &PPSNAL) {
        // Remove if same id
        if let Some(existing_pps) = self.pps.get(pps.pps_id as usize) {
            if existing_pps == pps {
                self.pps.remove(pps.pps_id as usize);
            }
        }
    }

    // If we're here, the last slice of a frame was found already
    fn add_current_frame(&mut self) {
        if self.current_frame.first_slice.first_slice_in_pic_flag {
            self.decoded_index += 1;

            self.current_frame.presentation_number =
                self.current_frame.first_slice.output_picture_number;

            self.current_frame.frame_type = self.current_frame.first_slice.slice_type;

            self.frames.push(self.current_frame.clone());

            self.current_frame = Frame::default();
        }
    }

    fn reorder_frames(&mut self) {
        let mut offset = self.presentation_index;

        self.frames.sort_by_key(|f| f.presentation_number);
        self.frames.iter_mut().for_each(|f| {
            f.presentation_number = offset;
            offset += 1;
        });

        self.presentation_index = offset;
        self.ordered_frames.extend_from_slice(&self.frames);
        self.frames.clear();
    }

    pub fn display(&self) {
        println!("{} frames", &self.ordered_frames.len());
        for frame in &self.ordered_frames {
            let pict_type = match frame.frame_type {
                2 => "I",
                1 => "P",
                0 => "B",
                _ => "",
            };

            println!(
                "{} display order {} poc {} pos {}",
                pict_type,
                frame.presentation_number,
                frame.first_slice.output_picture_number,
                frame.decoded_number
            );
        }
    }

    pub fn finish(&mut self) {
        self.add_current_frame();
        self.reorder_frames();
    }

    pub fn ordered_frames(&self) -> &Vec<Frame> {
        &self.ordered_frames
    }
}