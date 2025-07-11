use anyhow::Result;
use bitvec_helpers::bitstream_io_reader::BsIoSliceReader;

/// HEVC Decoder Configuration Record
/// ISO/IEC 14496-15
#[derive(Default, Debug, PartialEq, Clone, Eq)]
pub struct HEVCDecoderConfigurationRecord {
    pub configuration_version: u8,
    pub general_profile_space: u8,
    pub general_tier_flag: bool,
    pub general_profile_idc: u8,
    pub general_profile_compatibility_flags: u32,
    pub general_constraint_indicator_flags: u64,
    pub general_level_idc: u8,
    pub min_spatial_segmentation_idc: u16,
    pub parallelism_type: u8,
    pub chroma_format_idc: u8,
    pub bit_depth_luma_minus8: u8,
    pub bit_depth_chroma_minus8: u8,
    pub avg_frame_rate: u16,
    pub constant_frame_rate: u8,
    pub num_temporal_layers: u8,
    pub temporal_id_nested: bool,
    pub length_size_minus_one: u8,
    // nalu_arrays ignored
}

impl HEVCDecoderConfigurationRecord {
    pub fn parse(bs: &mut BsIoSliceReader) -> Result<Self> {
        let mut config = HEVCDecoderConfigurationRecord {
            configuration_version: bs.read::<8, u8>()?,
            general_profile_space: bs.read::<2, u8>()?,
            general_tier_flag: bs.read_bit()?,
            general_profile_idc: bs.read::<5, u8>()?,
            general_profile_compatibility_flags: bs.read::<32, u32>()?,
            general_constraint_indicator_flags: bs.read::<48, u64>()?,
            general_level_idc: bs.read::<8, u8>()?,
            ..Default::default()
        };

        bs.skip_n(4)?; // reserved 4bits
        config.min_spatial_segmentation_idc = bs.read::<12, u16>()?;

        bs.skip_n(6)?; // reserved 6 bits
        config.parallelism_type = bs.read::<2, u8>()?;

        bs.skip_n(6)?; // reserved 6 bits
        config.chroma_format_idc = bs.read::<2, u8>()?;

        bs.skip_n(5)?; // reserved 5 bits
        config.bit_depth_luma_minus8 = bs.read::<3, u8>()?;

        bs.skip_n(5)?; // reserved 5 bits
        config.bit_depth_chroma_minus8 = bs.read::<3, u8>()?;

        config.avg_frame_rate = bs.read::<16, u16>()?;
        config.constant_frame_rate = bs.read::<2, u8>()?;
        config.num_temporal_layers = bs.read::<3, u8>()?;
        config.temporal_id_nested = bs.read_bit()?;
        config.length_size_minus_one = bs.read::<2, u8>()?;

        Ok(config)
    }
}
