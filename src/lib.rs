use bitvec_helpers::bitvec_reader::BitVecReader;

pub mod hevc;
pub mod utils;

use hevc::*;
use vps::VPSNal;
use sps::SPSNal;
use pps::PPSNal;

use utils::clear_start_code_emulation_prevention_3_byte;

// We don't want to parse large slices because the memory is copied
const MAX_PARSE_SIZE: usize = 2048;

#[derive(Default)]
pub struct HevcParser {
    nals: Vec<NalUnit>,
    vps: Vec<VPSNal>,
    sps: Vec<SPSNal>,
    pps: Vec<PPSNal>,

    reader: BitVecReader,
}

impl HevcParser {
    pub fn parse_nal(&mut self, data: &[u8], offset: usize, size: usize) -> NalUnit {
        let mut nal = NalUnit::default();

        // Assuming [0, 0, 0, 1] header
        // Offset is at first element
        let pos = offset + 3;
        let max_size = if size > MAX_PARSE_SIZE {
            MAX_PARSE_SIZE
        } else {
            size
        };

        nal.start = pos;
        nal.end = pos + size;

        let bytes = clear_start_code_emulation_prevention_3_byte(&data[pos..pos + max_size]);
        self.reader = BitVecReader::new(bytes);

        self.parse_nal_header(&mut nal);

        self.nals.push(nal.clone());

        if nal.nuh_layer_id > 0 {
            return nal;
        }

        // ID by type
        nal.id = match nal.nal_type {
            NAL_VPS => self.parse_vps(),
            NAL_SPS => self.parse_sps(),
            NAL_PPS => self.parse_pps(),

            NAL_TRAIL_R | NAL_TRAIL_N | NAL_TSA_N | NAL_TSA_R |
            NAL_STSA_N | NAL_STSA_R | NAL_BLA_W_LP | NAL_BLA_W_RADL |
            NAL_BLA_N_LP | NAL_IDR_W_RADL | NAL_IDR_N_LP | NAL_CRA_NUT |
            NAL_RADL_N | NAL_RADL_R | NAL_RASL_N | NAL_RASL_R => {
                self.parse_slice()
            },
            _ => None,
        };

        nal
    }

    fn parse_nal_header(&mut self, nal: &mut NalUnit) {
        // forbidden_zero_bit
        self.reader.get();

        nal.nal_type = self.reader.get_n(6);
        nal.nuh_layer_id = self.reader.get_n(6);
        nal.temporal_id = self.reader.get_n::<u8>(3) - 1;
    }
    
    fn parse_vps(&mut self) -> Option<usize> {
        let vps = VPSNal::parse(&mut self.reader);
        let id = Some(vps.vps_id as usize);

        self.remove_vps(&vps);

        self.vps.push(vps);

        id
    }
    
    fn parse_sps(&mut self) -> Option<usize> {
        let sps = SPSNal::parse(&mut self.reader);
        let id = Some(sps.sps_id as usize);

        self.remove_sps(&sps);

        self.sps.push(sps);

        id
    }

    fn parse_pps(&mut self) -> Option<usize> {
        let pps = PPSNal::parse(&mut self.reader, &self.sps);
        let id = Some(pps.pps_id as usize);

        self.remove_pps(&pps);

        self.pps.push(pps);

        id
    }

    fn parse_slice(&mut self) -> Option<usize> {
        let id = Some(0);
        
        id
    }

    fn remove_vps(&mut self, vps: &VPSNal) {
        let id = vps.vps_id as usize;

        if let Some(existing_vps) = self.vps.get(id) {
            if existing_vps == vps {
                self.vps.remove(id);

                let sps_to_remove: Vec<SPSNal> = self.sps.clone()
                    .into_iter()
                    .filter(|sps| sps.vps_id == vps.vps_id)
                    .collect();
                
                sps_to_remove.iter()
                    .for_each(|sps| self.remove_sps(sps));
            }
        }
    }

    fn remove_sps(&mut self, sps: &SPSNal) {
        let id = sps.sps_id as usize;

        if let Some(existing_sps) = self.sps.get(id) {
            if existing_sps == sps {
                self.sps.remove(id);

                // Remove all dependent pps
                self.pps.retain(|pps| pps.sps_id != sps.sps_id);
            }
        }
    }

    fn remove_pps(&mut self, pps: &PPSNal) {
        // Remove if same id
        if let Some(existing_pps) = self.pps.get(pps.pps_id as usize) {
            if existing_pps == pps {
                self.pps.remove(pps.pps_id as usize);
            }
        }
    }
}