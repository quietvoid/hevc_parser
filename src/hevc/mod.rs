use std::fmt;

use self::slice::SliceNAL;

use super::{BsIoVecReader, NALUStartCode};

pub mod context;
pub(crate) mod hrd_parameters;
pub mod hvcc;
pub mod pps;
pub(crate) mod profile_tier_level;
pub(crate) mod scaling_list_data;
pub mod sei;
pub(crate) mod short_term_rps;
pub(crate) mod slice;
pub mod sps;
pub mod vps;
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

#[derive(PartialEq, Hash, Debug, Copy, Clone)]
pub enum UnitType {
    NalTrailN,
    NalTrailR,
    NalTsaN,
    NalTsaR,
    NalStsaN,
    NalStsaR,
    NalRadlN,
    NalRadlR,
    NalRaslN,
    NalRaslR,
    NalRsvVclN(u8),
    NalRsvVclR(u8),
    NalBlaWLp,
    NalBlaWRadl,
    NalBlaNLp,
    NalIdrWRadl,
    NalIdrNLp,
    NalCraNut,
    NalRsvIrapVcl(u8),
    NalRsvVcl(u8),
    NalVps,
    NalSps,
    NalPps,
    NalAud,
    NalEosNut,
    NalEobNut,
    NalFdNut,
    NalSeiPrefix,
    NalSeiSuffix,
    RsvNvcl(u8),
    NalUnspec(u8),
}
impl UnitType {
    pub fn for_id(id: u8) -> Result<UnitType, UnitTypeError> {
        let t = match id {
            0 => UnitType::NalTrailN,
            1 => UnitType::NalTrailR,
            2 => UnitType::NalTsaN,
            3 => UnitType::NalTsaR,
            4 => UnitType::NalStsaN,
            5 => UnitType::NalStsaR,
            6 => UnitType::NalRadlN,
            7 => UnitType::NalRadlR,
            8 => UnitType::NalRaslN,
            9 => UnitType::NalRaslR,
            10 | 12 | 14 => UnitType::NalRsvVclN(id),
            11 | 13 | 15 => UnitType::NalRsvVclR(id),
            16 => UnitType::NalBlaWLp,
            17 => UnitType::NalBlaWRadl,
            18 => UnitType::NalBlaNLp,
            19 => UnitType::NalIdrWRadl,
            20 => UnitType::NalIdrNLp,
            21 => UnitType::NalCraNut,
            22 | 23 => UnitType::NalRsvIrapVcl(id),
            24..=31 => UnitType::NalRsvVcl(id),
            32 => UnitType::NalVps,
            33 => UnitType::NalSps,
            34 => UnitType::NalPps,
            35 => UnitType::NalAud,
            36 => UnitType::NalEosNut,
            37 => UnitType::NalEobNut,
            38 => UnitType::NalFdNut,
            39 => UnitType::NalSeiPrefix,
            40 => UnitType::NalSeiSuffix,
            41..=47 => UnitType::RsvNvcl(id),
            48..=63 => UnitType::NalUnspec(id),
            _ => return Err(UnitTypeError::ValueOutOfRange(id)),
        };

        Ok(t)
    }

    pub fn id(self) -> u8 {
        match self {
            UnitType::NalTrailN => 0,
            UnitType::NalTrailR => 1,
            UnitType::NalTsaN => 2,
            UnitType::NalTsaR => 3,
            UnitType::NalStsaN => 4,
            UnitType::NalStsaR => 5,
            UnitType::NalRadlN => 6,
            UnitType::NalRadlR => 7,
            UnitType::NalRaslN => 8,
            UnitType::NalRaslR => 9,
            UnitType::NalRsvVclN(v) => v,
            UnitType::NalRsvVclR(v) => v,
            UnitType::NalBlaWLp => 16,
            UnitType::NalBlaWRadl => 17,
            UnitType::NalBlaNLp => 18,
            UnitType::NalIdrWRadl => 19,
            UnitType::NalIdrNLp => 20,
            UnitType::NalCraNut => 21,
            UnitType::NalRsvIrapVcl(v) => v,
            UnitType::NalRsvVcl(v) => v,
            UnitType::NalVps => 32,
            UnitType::NalSps => 33,
            UnitType::NalPps => 34,
            UnitType::NalAud => 35,
            UnitType::NalEosNut => 36,
            UnitType::NalEobNut => 37,
            UnitType::NalFdNut => 38,
            UnitType::NalSeiPrefix => 39,
            UnitType::NalSeiSuffix => 40,
            UnitType::RsvNvcl(v) => v,
            UnitType::NalUnspec(v) => v,
        }
    }
}

#[derive(Debug)]
pub enum UnitTypeError {
    /// if the value was outside the range `0` - `63`.
    ValueOutOfRange(u8),
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub struct NalHeader(u16);

#[derive(Debug)]
pub enum NalHeaderError {
    /// The most significant bit of the header, called `forbidden_zero_bit`, was set to 1.
    ForbiddenZeroBit,
}
impl NalHeader {
    pub fn new(header_value: u16) -> Result<NalHeader, NalHeaderError> {
        if header_value & 0x8000 != 0 {
            Err(NalHeaderError::ForbiddenZeroBit)
        } else {
            Ok(NalHeader(header_value))
        }
    }

    pub fn nal_unit_type(self) -> UnitType {
        UnitType::for_id(((self.0 & 0x7E00) >> 9) as u8).unwrap()
    }

    pub fn nuh_layer_id(self) -> u8 {
        ((self.0 & 0x1F8) >> 3) as u8
    }

    pub fn nuh_temporal_id_plus1(self) -> u8 {
        (self.0 & 0x7) as u8
    }
}
impl From<NalHeader> for u16 {
    fn from(v: NalHeader) -> Self {
        v.0
    }
}

impl fmt::Debug for NalHeader {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        f.debug_struct("NalHeader")
            .field("nal_unit_type", &self.nal_unit_type())
            .field("nuh_layer_id", &self.nuh_layer_id())
            .field("nuh_temporal_id_plus1", &self.nuh_temporal_id_plus1())
            .finish()
    }
}

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
