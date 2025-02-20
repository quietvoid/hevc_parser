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
            configuration_version: bs.get_n(8)?,
            general_profile_space: bs.get_n(2)?,
            general_tier_flag: bs.get()?,
            general_profile_idc: bs.get_n(5)?,
            general_profile_compatibility_flags: bs.get_n(32)?,
            general_constraint_indicator_flags: bs.get_n(48)?,
            general_level_idc: bs.get_n(8)?,
            ..Default::default()
        };

        bs.skip_n(4)?; // reserved 4bits
        config.min_spatial_segmentation_idc = bs.get_n(12)?;

        bs.skip_n(6)?; // reserved 6 bits
        config.parallelism_type = bs.get_n(2)?;

        bs.skip_n(6)?; // reserved 6 bits
        config.chroma_format_idc = bs.get_n(2)?;

        bs.skip_n(5)?; // reserved 5 bits
        config.bit_depth_luma_minus8 = bs.get_n(3)?;

        bs.skip_n(5)?; // reserved 5 bits
        config.bit_depth_chroma_minus8 = bs.get_n(3)?;

        config.avg_frame_rate = bs.get_n(16)?;
        config.constant_frame_rate = bs.get_n(2)?;
        config.num_temporal_layers = bs.get_n(3)?;
        config.temporal_id_nested = bs.get()?;
        config.length_size_minus_one = bs.get_n(2)?;

        Ok(config)
    }
}
