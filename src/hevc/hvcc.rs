//! Support for handling _High Efficiency Video Coding Configuration_ data, used in the _ISO Base Media
//! File Format_ (AKA MP4)
//!

use crate::{
    context::Context,
    hevc::{pps, sps, vps, UnitType},
    NalHeader, NalHeaderError,
};
use std::convert::TryFrom;

#[derive(Debug)]
pub enum HvccError {
    Io(std::io::Error),
    NotEnoughData {
        expected: usize,
        actual: usize,
    },
    /// The HevcDecoderConfigurationRecord used a version number other than `1`.
    UnsupportedConfigurationVersion(u8),
    ParamSet(ParamSetError),
}

pub struct HevcDecoderConfigurationRecord<'buf> {
    data: &'buf [u8],
}
impl<'buf> TryFrom<&'buf [u8]> for HevcDecoderConfigurationRecord<'buf> {
    type Error = HvccError;

    fn try_from(data: &'buf [u8]) -> Result<Self, Self::Error> {
        let hvcc = HevcDecoderConfigurationRecord { data };
        // we must confirm we have enough bytes for all fixed fields before we do anything else,
        hvcc.ck(Self::MIN_CONF_SIZE)?;
        if hvcc.configuration_version() != 1 {
            // The spec requires that decoders ignore streams where the version number is not 1,
            // indicating there was an incompatible change in the configuration format,
            return Err(HvccError::UnsupportedConfigurationVersion(
                hvcc.configuration_version(),
            ));
        }

        Ok(hvcc)
    }
}
impl<'buf> HevcDecoderConfigurationRecord<'buf> {
    const MIN_CONF_SIZE: usize = 23;

    fn ck(&self, len: usize) -> Result<(), HvccError> {
        if self.data.len() < len {
            Err(HvccError::NotEnoughData {
                expected: len,
                actual: self.data.len(),
            })
        } else {
            Ok(())
        }
    }
    pub fn configuration_version(&self) -> u8 {
        self.data[0]
    }
    pub fn num_of_arrays(&self) -> usize {
        self.data[22] as usize
    }
    pub fn general_profile_idc(&self) -> u8 {
        self.data[1] & 0b0001_1111
    }
    pub fn general_profile_compatibility_flags(&self) -> u32 {
        (self.data[2] as u32) << 3
            | (self.data[3] as u32) << 2
            | (self.data[4] as u32) << 1
            | self.data[5] as u32
    }
    pub fn general_level_idc(&self) -> u8 {
        self.data[12]
    }
    /// Number of bytes used to specify the length of each NAL unit
    /// 0 => 1 byte, 1 => 2 bytes, 2 => 3 bytes, 3 => 4 bytes
    pub fn length_size_minus_one(&self) -> u8 {
        self.data[21] & 0b0000_0011
    }
    pub fn parameter_sets(
        &self,
        unit_type: UnitType,
    ) -> impl Iterator<Item = Result<&'buf [u8], ParamSetError>> {
        let mut data = &self.data[Self::MIN_CONF_SIZE..];
        let num_arrays = self.num_of_arrays();

        for _ in 0..num_arrays {
            let nal_type = data[0] & 0b0011_1111;
            let num_nalus = u16::from(data[1]) << 8 | u16::from(data[2]);
            data = &data[3..];
            if nal_type == unit_type.id() {
                return ParamSetIter::new(data, unit_type).take(num_nalus as usize);
            }
            for _ in 0..num_nalus {
                let nal_unit_len = u16::from(data[0]) << 8 | u16::from(data[1]);
                let offset = nal_unit_len as usize + 2;
                data = &data[offset..];
            }
        }

        ParamSetIter::new(data, unit_type).take(0)
    }

    /// Creates an H265 parser context, using the settings encoded into
    /// this `HevcDecoderConfigurationRecord`.
    ///
    /// In particular, the _sequence parameter set_ and _picture parameter set_ values of this
    /// configuration record will be inserted into the resulting context.
    pub fn create_context(&self) -> Result<Context, HvccError> {
        let mut ctx = Context::new();
        for vps in self.parameter_sets(UnitType::NalVps) {
            let vps = vps.map_err(HvccError::ParamSet)?;
            let vps = crate::clear_start_code_emulation_prevention_3_byte(vps);
            let mut reader = bitvec_helpers::bitstream_io_reader::BsIoVecReader::from_vec(vps);
            reader.get_n::<u16>(16).map_err(|e| {
                HvccError::ParamSet(ParamSetError::Parse(format!(
                    "failed to read vps header: {e}"
                )))
            })?;
            let vps = vps::VPSNAL::parse(&mut reader).map_err(|err| {
                HvccError::ParamSet(ParamSetError::Parse(format!("failed to parse vps: {err}")))
            })?;
            ctx.put_vid_param_set(vps);
        }
        for sps in self.parameter_sets(UnitType::NalSps) {
            let sps = sps.map_err(HvccError::ParamSet)?;
            let sps = crate::clear_start_code_emulation_prevention_3_byte(sps);
            let mut reader = bitvec_helpers::bitstream_io_reader::BsIoVecReader::from_vec(sps);
            reader.get_n::<u16>(16).map_err(|e| {
                HvccError::ParamSet(ParamSetError::Parse(format!(
                    "failed to read sps header: {e}"
                )))
            })?;
            let sps = sps::SPSNAL::parse(&mut reader).map_err(|err| {
                HvccError::ParamSet(ParamSetError::Parse(format!("failed to parse sps: {err}")))
            })?;
            ctx.put_seq_param_set(sps);
        }
        for pps in self.parameter_sets(UnitType::NalPps) {
            let pps = pps.map_err(HvccError::ParamSet)?;
            let pps = crate::clear_start_code_emulation_prevention_3_byte(pps);
            let mut reader = bitvec_helpers::bitstream_io_reader::BsIoVecReader::from_vec(pps);
            reader.get_n::<u16>(16).map_err(|e| {
                HvccError::ParamSet(ParamSetError::Parse(format!(
                    "failed to read pps header: {e}"
                )))
            })?;
            let pps = pps::PPSNAL::parse(&mut reader).map_err(|err| {
                HvccError::ParamSet(ParamSetError::Parse(format!("failed to parse pps: {err}")))
            })?;
            ctx.put_pic_param_set(pps);
        }
        Ok(ctx)
    }
}

#[derive(Debug)]
pub enum ParamSetError {
    NalHeader(NalHeaderError),
    IncorrectNalType {
        expected: UnitType,
        actual: UnitType,
    },
    Parse(String),
}

struct ParamSetIter<'buf>(&'buf [u8], UnitType);

impl<'buf> ParamSetIter<'buf> {
    pub fn new(buf: &'buf [u8], unit_type: UnitType) -> ParamSetIter<'buf> {
        ParamSetIter(buf, unit_type)
    }
}
impl<'buf> Iterator for ParamSetIter<'buf> {
    type Item = Result<&'buf [u8], ParamSetError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.0.is_empty() {
            None
        } else {
            let len = u16::from(self.0[0]) << 8 | u16::from(self.0[1]);
            let data = &self.0[2..];
            let res = match NalHeader::new(u16::from(self.0[2]) << 8 | u16::from(self.0[3])) {
                Ok(nal_header) => {
                    if nal_header.nal_unit_type() == self.1 {
                        let (data, remainder) = data.split_at(len as usize);
                        self.0 = remainder;
                        Ok(data)
                    } else {
                        Err(ParamSetError::IncorrectNalType {
                            expected: self.1,
                            actual: nal_header.nal_unit_type(),
                        })
                    }
                }
                Err(err) => Err(ParamSetError::NalHeader(err)),
            };
            Some(res)
        }
    }
}
