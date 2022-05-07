use anyhow::{bail, Result};

use self::slice::SliceNAL;

use super::{BitVecReader, NALUStartCode};

pub(crate) mod hrd_parameters;
pub(crate) mod pps;
pub(crate) mod profile_tier_level;
pub(crate) mod scaling_list_data;
pub(crate) mod short_term_rps;
pub(crate) mod slice;
pub(crate) mod sps;
pub(crate) mod vps;
pub(crate) mod vui_parameters;

// https://github.com/virinext/hevcesbrowser/blob/master/hevcparser/include/Hevc.h
pub const NAL_TRAIL_N: u8 = 0;
pub const NAL_TRAIL_R: u8 = 1;
pub const NAL_TSA_N: u8 = 2;
pub const NAL_TSA_R: u8 = 3;
pub const NAL_STSA_N: u8 = 4;
pub const NAL_STSA_R: u8 = 5;
pub const NAL_RADL_N: u8 = 6;
pub const NAL_RADL_R: u8 = 7;
pub const NAL_RASL_N: u8 = 8;
pub const NAL_RASL_R: u8 = 9;
pub const NAL_BLA_W_LP: u8 = 16;
pub const NAL_BLA_W_RADL: u8 = 17;
pub const NAL_BLA_N_LP: u8 = 18;
pub const NAL_IDR_W_RADL: u8 = 19;
pub const NAL_IDR_N_LP: u8 = 20;
pub const NAL_CRA_NUT: u8 = 21;
pub const NAL_IRAP_VCL23: u8 = 23;
pub const NAL_VPS: u8 = 32;
pub const NAL_SPS: u8 = 33;
pub const NAL_PPS: u8 = 34;
pub const NAL_AUD: u8 = 35;
pub const NAL_EOS_NUT: u8 = 36;
pub const NAL_EOB_NUT: u8 = 37;
pub const NAL_FD_NUT: u8 = 38;
pub const NAL_SEI_PREFIX: u8 = 39;
pub const NAL_SEI_SUFFIX: u8 = 40;
pub const NAL_UNSPEC62: u8 = 62;
pub const NAL_UNSPEC63: u8 = 63;

pub const USER_DATA_REGISTERED_ITU_T_35: u8 = 4;

#[derive(Default, Debug, Clone)]
pub struct NALUnit {
    pub start: usize,
    pub end: usize,

    pub nal_type: u8,
    pub nuh_layer_id: u8,
    pub temporal_id: u8,

    pub start_code: NALUStartCode,

    #[deprecated(since = "0.4.0", note = "Please use `start_code` instead")]
    pub start_code_len: u8,

    pub decoded_frame_index: u64,
}

#[derive(Default, Debug, Clone)]
pub struct Frame {
    pub decoded_number: u64,
    pub presentation_number: u64,
    pub frame_type: u64,

    pub nals: Vec<NALUnit>,
    pub first_slice: SliceNAL,
}

#[derive(Default, Debug, Clone)]
pub struct SeiMessage {
    num_payload_type_ff_bytes: usize,
    last_payload_type_byte: u8,

    num_payload_size_ff_bytes: usize,
    last_payload_size_byte: u8,

    pub payload_type: u8,
    pub payload_size: usize,
}

impl SeiMessage {
    pub fn from_bytes(data: &[u8]) -> Result<SeiMessage> {
        let mut reader = BitVecReader::new(data.to_vec());

        SeiMessage::parse(&mut reader)
    }

    pub fn parse(reader: &mut BitVecReader) -> Result<SeiMessage> {
        // forbidden_zero_bit
        reader.skip_n(1);

        let nal_type = reader.get_n::<u8>(6);

        if nal_type != NAL_SEI_PREFIX {
            bail!("NAL type {} is not SEI_PREFIX", nal_type);
        }

        if reader.available() < 9 && matches!(nal_type, NAL_EOS_NUT | NAL_EOB_NUT) {
        } else {
            reader.skip_n(6); // nuh_layer_id
            reader.skip_n(3); // temporal_id
        }

        let mut msg = SeiMessage {
            last_payload_type_byte: reader.get_n(8),
            ..Default::default()
        };

        while msg.last_payload_type_byte == 0xFF {
            msg.num_payload_type_ff_bytes += 1;
            msg.last_payload_type_byte = reader.get_n(8);

            msg.payload_type += 255;
        }

        msg.payload_type += msg.last_payload_type_byte;

        msg.last_payload_size_byte = reader.get_n(8);
        while msg.last_payload_size_byte == 0xFF {
            msg.num_payload_size_ff_bytes += 1;
            msg.last_payload_size_byte = reader.get_n(8);

            msg.payload_size += 255;
        }

        msg.payload_size += msg.last_payload_size_byte as usize;

        if msg.payload_size > reader.available() {
            bail!("Payload size is larger than NALU size");
        }

        Ok(msg)
    }
}

impl NALUnit {
    pub fn is_type_slice(nal_type: u8) -> bool {
        matches!(
            nal_type,
            NAL_TRAIL_R
                | NAL_TRAIL_N
                | NAL_TSA_N
                | NAL_TSA_R
                | NAL_STSA_N
                | NAL_STSA_R
                | NAL_BLA_W_LP
                | NAL_BLA_W_RADL
                | NAL_BLA_N_LP
                | NAL_IDR_W_RADL
                | NAL_IDR_N_LP
                | NAL_CRA_NUT
                | NAL_RADL_N
                | NAL_RADL_R
                | NAL_RASL_N
                | NAL_RASL_R
        )
    }

    pub fn is_slice(&self) -> bool {
        Self::is_type_slice(self.nal_type)
    }
}
