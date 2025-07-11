use anyhow::Result;

use super::BsIoVecReader;
use super::profile_tier_level::ProfileTierLevel;
use super::scaling_list_data::ScalingListData;
use super::short_term_rps::ShortTermRPS;
use super::vui_parameters::VuiParameters;

#[allow(clippy::upper_case_acronyms)]
#[derive(Default, Debug, PartialEq, Clone, Eq)]
pub struct SPSNAL {
    pub(crate) vps_id: u8,
    max_sub_layers: u8,
    temporal_id_nesting_flag: bool,

    ptl: ProfileTierLevel,
    pub(crate) sps_id: u64,
    chroma_format_idc: u64,
    pub(crate) separate_colour_plane_flag: bool,
    width: u64,
    height: u64,

    pic_conformance_flag: bool,
    conf_win_left_offset: u64,
    conf_win_right_offset: u64,
    conf_win_top_offset: u64,
    conf_win_bottom_offset: u64,

    bit_depth: u64,
    bit_depth_chroma: u64,
    pub(crate) log2_max_poc_lsb: u64,
    sublayer_ordering_info: bool,
    max_dec_pic_buffering: Vec<u64>,
    num_reorder_pics: Vec<u64>,
    max_latency_increase: Vec<u64>,

    log2_min_cb_size: u64,
    log2_diff_max_min_coding_block_size: u64,
    log2_min_tb_size: u64,
    log2_diff_max_min_transform_block_size: u64,
    max_transform_hierarchy_depth_inter: u64,
    max_transform_hierarchy_depth_intra: u64,

    scaling_list_enabled_flag: bool,
    scaling_list_data_present_flag: bool,
    scaling_list_data: ScalingListData,

    amp_enabled_flag: bool,
    sao_enabled_flag: bool,
    pcm_enabled_flag: bool,
    pcm_bit_depth: u8,
    pcm_bit_depth_chroma: u8,
    pcm_log2_min_pcm_cb_size: u64,
    pcm_log2_max_pcm_cb_size: u64,
    pcm_loop_filter_disable_flag: bool,

    nb_st_rps: u64,
    pub(crate) short_term_ref_pic_sets: Vec<ShortTermRPS>,

    long_term_ref_pics_present_flag: bool,
    num_long_term_ref_pics_sps: u64,
    lt_ref_pic_poc_lsb_sps: Vec<u64>,
    used_by_curr_pic_lt_sps_flag: Vec<bool>,

    sps_temporal_mvp_enabled_flag: bool,
    sps_strong_intra_smoothing_enable_flag: bool,

    vui_present: bool,
    vui_parameters: VuiParameters,

    sps_extension_flag: bool,

    // Computed values
    pub(crate) log2_ctb_size: u64,
    pub(crate) log2_min_pu_size: u64,
    pub(crate) ctb_width: u64,
    pub(crate) ctb_height: u64,
    pub(crate) ctb_size: u64,
    pub(crate) min_cb_width: u64,
    pub(crate) min_cb_height: u64,
    pub(crate) min_tb_width: u64,
    pub(crate) min_tb_height: u64,
    pub(crate) min_pu_width: u64,
    pub(crate) min_pu_height: u64,
    pub(crate) tb_mask: u64,
}

impl SPSNAL {
    pub fn parse(bs: &mut BsIoVecReader) -> Result<SPSNAL> {
        let mut sps = SPSNAL {
            vps_id: bs.read::<4, u8>()?,
            ..Default::default()
        };

        sps.max_sub_layers = bs.read::<3, u8>()? + 1;
        sps.temporal_id_nesting_flag = bs.read_bit()?;

        sps.ptl.parse(bs, sps.max_sub_layers)?;

        sps.sps_id = bs.read_ue()?;
        sps.chroma_format_idc = bs.read_ue()?;

        if sps.chroma_format_idc == 3 {
            sps.separate_colour_plane_flag = bs.read_bit()?;
        }

        if sps.separate_colour_plane_flag {
            sps.chroma_format_idc = 0;
        }

        sps.width = bs.read_ue()?;
        sps.height = bs.read_ue()?;
        sps.pic_conformance_flag = bs.read_bit()?;

        if sps.pic_conformance_flag {
            sps.conf_win_left_offset = bs.read_ue()?;
            sps.conf_win_right_offset = bs.read_ue()?;
            sps.conf_win_top_offset = bs.read_ue()?;
            sps.conf_win_bottom_offset = bs.read_ue()?;
        }

        sps.bit_depth = bs.read_ue()? + 8;
        sps.bit_depth_chroma = bs.read_ue()? + 8;
        sps.log2_max_poc_lsb = bs.read_ue()? + 4;
        sps.sublayer_ordering_info = bs.read_bit()?;

        let start = if sps.sublayer_ordering_info {
            0
        } else {
            sps.max_sub_layers - 1
        };

        for _ in start..sps.max_sub_layers {
            sps.max_dec_pic_buffering.push(bs.read_ue()? + 1);
            sps.num_reorder_pics.push(bs.read_ue()?);

            let mut max_latency_increase = bs.read_ue()?;
            max_latency_increase = max_latency_increase.saturating_sub(1);

            sps.max_latency_increase.push(max_latency_increase);
        }

        sps.log2_min_cb_size = bs.read_ue()? + 3;
        sps.log2_diff_max_min_coding_block_size = bs.read_ue()?;
        sps.log2_min_tb_size = bs.read_ue()? + 2;
        sps.log2_diff_max_min_transform_block_size = bs.read_ue()?;

        sps.max_transform_hierarchy_depth_inter = bs.read_ue()?;
        sps.max_transform_hierarchy_depth_intra = bs.read_ue()?;

        sps.scaling_list_enabled_flag = bs.read_bit()?;

        if sps.scaling_list_enabled_flag {
            sps.scaling_list_data_present_flag = bs.read_bit()?;

            if sps.scaling_list_data_present_flag {
                sps.scaling_list_data = ScalingListData::parse(bs)?;
            }
        }

        sps.amp_enabled_flag = bs.read_bit()?;
        sps.sao_enabled_flag = bs.read_bit()?;
        sps.pcm_enabled_flag = bs.read_bit()?;

        if sps.pcm_enabled_flag {
            sps.pcm_bit_depth = bs.read::<4, u8>()? + 1;
            sps.pcm_bit_depth_chroma = bs.read::<4, u8>()? + 1;
            sps.pcm_log2_min_pcm_cb_size = bs.read_ue()? + 3;
            sps.pcm_log2_max_pcm_cb_size = bs.read_ue()? + sps.pcm_log2_min_pcm_cb_size;

            sps.pcm_loop_filter_disable_flag = bs.read_bit()?;
        }

        sps.nb_st_rps = bs.read_ue()?;

        sps.short_term_ref_pic_sets
            .resize_with(sps.nb_st_rps as usize, Default::default);
        for i in 0..sps.nb_st_rps as usize {
            sps.short_term_ref_pic_sets[i] =
                ShortTermRPS::parse(bs, &sps, i, sps.nb_st_rps, false)?;
        }

        sps.long_term_ref_pics_present_flag = bs.read_bit()?;

        if sps.long_term_ref_pics_present_flag {
            sps.num_long_term_ref_pics_sps = bs.read_ue()?;

            for _ in 0..sps.num_long_term_ref_pics_sps {
                sps.lt_ref_pic_poc_lsb_sps
                    .push(bs.read_var(sps.log2_max_poc_lsb as u32)?);
                sps.used_by_curr_pic_lt_sps_flag.push(bs.read_bit()?);
            }
        }

        sps.sps_temporal_mvp_enabled_flag = bs.read_bit()?;
        sps.sps_strong_intra_smoothing_enable_flag = bs.read_bit()?;

        sps.vui_present = bs.read_bit()?;

        if sps.vui_present {
            sps.vui_parameters = VuiParameters::parse(bs, sps.max_sub_layers)?;
        }

        sps.sps_extension_flag = bs.read_bit()?;

        // Computed values
        sps.log2_ctb_size = sps.log2_min_cb_size + sps.log2_diff_max_min_coding_block_size;
        sps.log2_min_pu_size = sps.log2_min_cb_size - 1;

        sps.ctb_width = (sps.width + (1 << sps.log2_ctb_size) - 1) >> sps.log2_ctb_size;
        sps.ctb_height = (sps.height + (1 << sps.log2_ctb_size) - 1) >> sps.log2_ctb_size;
        sps.ctb_size = sps.ctb_width * sps.ctb_height;

        sps.min_cb_width = sps.width >> sps.log2_min_cb_size;
        sps.min_cb_height = sps.height >> sps.log2_min_cb_size;
        sps.min_tb_width = sps.width >> sps.log2_min_tb_size;
        sps.min_tb_height = sps.height >> sps.log2_min_tb_size;
        sps.min_pu_width = sps.width >> sps.log2_min_pu_size;
        sps.min_pu_height = sps.height >> sps.log2_min_pu_size;
        sps.tb_mask = (1 << (sps.log2_ctb_size - sps.log2_min_tb_size)) - 1;

        Ok(sps)
    }
}
