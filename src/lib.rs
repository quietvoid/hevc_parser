use bitvec_helpers::bitvec_reader::BitVecReader;

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
    pub fn parse_nal(&mut self, data: &[u8], offset: usize, size: usize) -> NALUnit {
        let mut nal = NALUnit::default();

        // Assuming [0, 0, 0, 1] header
        // Offset is at first element
        let pos = offset + 3;

        let end = if size > MAX_PARSE_SIZE {
            pos + MAX_PARSE_SIZE
        } else if pos + size >= data.len() {
            offset + size
        } else {
            pos + size
        };

        nal.start = pos;
        nal.end = end;
        nal.decoded_frame_index = self.decoded_index;

        let bytes = clear_start_code_emulation_prevention_3_byte(&data[pos..end]);
        self.reader = BitVecReader::new(bytes);

        self.parse_nal_header(&mut nal);

        self.nals.push(nal.clone());

        if nal.nuh_layer_id > 0 {
            return nal;
        }

        match nal.nal_type {
            NAL_VPS => self.parse_vps(),
            NAL_SPS => self.parse_sps(),
            NAL_PPS => self.parse_pps(),

            NAL_TRAIL_R | NAL_TRAIL_N | NAL_TSA_N | NAL_TSA_R | NAL_STSA_N | NAL_STSA_R
            | NAL_BLA_W_LP | NAL_BLA_W_RADL | NAL_BLA_N_LP | NAL_IDR_W_RADL | NAL_IDR_N_LP
            | NAL_CRA_NUT | NAL_RADL_N | NAL_RADL_R | NAL_RASL_N | NAL_RASL_R => {
                self.current_frame.nals.push(nal.clone());
                
                self.parse_slice(&nal)
            }
            _ => {
                let old_frame = self.current_frame.first_slice.first_slice_in_pic_flag;

                self.add_current_frame();

                let new_frame = !self.current_frame.first_slice.first_slice_in_pic_flag;

                if old_frame && new_frame {
                    self.current_frame.nals.push(nal.clone());
                }
            },
        };

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

    fn parse_slice(&mut self, nal: &NALUnit) {
        let slice = SliceNAL::parse(
            &mut self.reader,
            &self.sps,
            &self.pps,
            nal,
            &mut self.poc_tid0,
            &mut self.poc,
        );

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

            self.current_frame.presentation_number = self.current_frame.first_slice.output_picture_number;

            self.current_frame.frame_type = self.current_frame.first_slice.slice_type;

            self.frames.push(self.current_frame.clone());

            self.current_frame = Frame::default();
        }
    }

    fn reorder_frames(&mut self) {
        let mut offset = self.presentation_index;

        self.frames.sort_by_key(|f| f.presentation_number);
        self.frames
            .iter_mut()
            .for_each(|f| { 
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

            println!("{} display order {} poc {} pos {}", pict_type, frame.presentation_number, frame.first_slice.output_picture_number, frame.decoded_number);
        }
       // println!("{:#?}", self.ordered_frames);
    }

    pub fn finish(&mut self) {
        self.add_current_frame();
        self.reorder_frames();
    }
}
