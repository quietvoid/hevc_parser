use self::slice::SliceNAL;

use super::{BsIoVecReader, NALUStartCode};

pub mod config;
pub(crate) mod hrd_parameters;
pub(crate) mod pps;
pub(crate) mod profile_tier_level;
pub(crate) mod scaling_list_data;
pub mod sei;
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

pub use sei::SeiMessage;

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
