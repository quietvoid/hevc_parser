use anyhow::Result;

use super::profile_tier_level::ProfileTierLevel;
use super::scaling_list_data::ScalingListData;
use super::short_term_rps::ShortTermRPS;
use super::vui_parameters::VuiParameters;
use super::BsIoVecReader;

#[allow(clippy::upper_case_acronyms)]
#[derive(Default, Debug, PartialEq, Clone, Eq)]
pub struct SPSNAL {
    pub vps_id: u8,
    pub max_sub_layers: u8,
    pub temporal_id_nesting_flag: bool,

    pub ptl: ProfileTierLevel,
    pub sps_id: u64,
    pub chroma_format_idc: u64,
    pub separate_colour_plane_flag: bool,
    pub width: u64,
    pub height: u64,

    pub pic_conformance_flag: bool,
    pub conf_win_left_offset: u64,
    pub conf_win_right_offset: u64,
    pub conf_win_top_offset: u64,
    pub conf_win_bottom_offset: u64,

    pub bit_depth: u64,
    pub bit_depth_chroma: u64,
    pub log2_max_poc_lsb: u64,
    pub sublayer_ordering_info: bool,
    pub max_dec_pic_buffering: Vec<u64>,
    pub num_reorder_pics: Vec<u64>,
    pub max_latency_increase: Vec<u64>,

    pub log2_min_cb_size: u64,
    pub log2_diff_max_min_coding_block_size: u64,
    pub log2_min_tb_size: u64,
    pub log2_diff_max_min_transform_block_size: u64,
    pub max_transform_hierarchy_depth_inter: u64,
    pub max_transform_hierarchy_depth_intra: u64,

    pub scaling_list_enabled_flag: bool,
    pub scaling_list_data_present_flag: bool,
    pub scaling_list_data: ScalingListData,

    pub amp_enabled_flag: bool,
    pub sao_enabled_flag: bool,
    pub pcm_enabled_flag: bool,
    pub pcm_bit_depth: u8,
    pub pcm_bit_depth_chroma: u8,
    pub pcm_log2_min_pcm_cb_size: u64,
    pub pcm_log2_max_pcm_cb_size: u64,
    pub pcm_loop_filter_disable_flag: bool,

    pub nb_st_rps: u64,
    pub short_term_ref_pic_sets: Vec<ShortTermRPS>,

    pub long_term_ref_pics_present_flag: bool,
    pub num_long_term_ref_pics_sps: u64,
    pub lt_ref_pic_poc_lsb_sps: Vec<u64>,
    pub used_by_curr_pic_lt_sps_flag: Vec<bool>,

    pub sps_temporal_mvp_enabled_flag: bool,
    pub sps_strong_intra_smoothing_enable_flag: bool,

    pub vui_present: bool,
    pub vui_parameters: VuiParameters,

    pub sps_extension_flag: bool,

    // Computed values
    pub log2_ctb_size: u64,
    pub log2_min_pu_size: u64,
    pub ctb_width: u64,
    pub ctb_height: u64,
    pub ctb_size: u64,
    pub min_cb_width: u64,
    pub min_cb_height: u64,
    pub min_tb_width: u64,
    pub min_tb_height: u64,
    pub min_pu_width: u64,
    pub min_pu_height: u64,
    pub tb_mask: u64,
}

impl SPSNAL {
    pub fn parse(bs: &mut BsIoVecReader) -> Result<SPSNAL> {
        let mut sps = SPSNAL {
            vps_id: bs.get_n(4)?,
            ..Default::default()
        };

        sps.max_sub_layers = bs.get_n::<u8>(3)? + 1;
        sps.temporal_id_nesting_flag = bs.get()?;

        sps.ptl.parse(bs, sps.max_sub_layers)?;

        sps.sps_id = bs.get_ue()?;
        sps.chroma_format_idc = bs.get_ue()?;

        if sps.chroma_format_idc == 3 {
            sps.separate_colour_plane_flag = bs.get()?;
        }

        if sps.separate_colour_plane_flag {
            sps.chroma_format_idc = 0;
        }

        sps.width = bs.get_ue()?;
        sps.height = bs.get_ue()?;
        sps.pic_conformance_flag = bs.get()?;

        if sps.pic_conformance_flag {
            sps.conf_win_left_offset = bs.get_ue()?;
            sps.conf_win_right_offset = bs.get_ue()?;
            sps.conf_win_top_offset = bs.get_ue()?;
            sps.conf_win_bottom_offset = bs.get_ue()?;
        }

        sps.bit_depth = bs.get_ue()? + 8;
        sps.bit_depth_chroma = bs.get_ue()? + 8;
        sps.log2_max_poc_lsb = bs.get_ue()? + 4;
        sps.sublayer_ordering_info = bs.get()?;

        let start = if sps.sublayer_ordering_info {
            0
        } else {
            sps.max_sub_layers - 1
        };

        for _ in start..sps.max_sub_layers {
            sps.max_dec_pic_buffering.push(bs.get_ue()? + 1);
            sps.num_reorder_pics.push(bs.get_ue()?);

            let mut max_latency_increase = bs.get_ue()?;
            max_latency_increase = max_latency_increase.saturating_sub(1);

            sps.max_latency_increase.push(max_latency_increase);
        }

        sps.log2_min_cb_size = bs.get_ue()? + 3;
        sps.log2_diff_max_min_coding_block_size = bs.get_ue()?;
        sps.log2_min_tb_size = bs.get_ue()? + 2;
        sps.log2_diff_max_min_transform_block_size = bs.get_ue()?;

        sps.max_transform_hierarchy_depth_inter = bs.get_ue()?;
        sps.max_transform_hierarchy_depth_intra = bs.get_ue()?;

        sps.scaling_list_enabled_flag = bs.get()?;

        if sps.scaling_list_enabled_flag {
            sps.scaling_list_data_present_flag = bs.get()?;

            if sps.scaling_list_data_present_flag {
                sps.scaling_list_data = ScalingListData::parse(bs)?;
            }
        }

        sps.amp_enabled_flag = bs.get()?;
        sps.sao_enabled_flag = bs.get()?;
        sps.pcm_enabled_flag = bs.get()?;

        if sps.pcm_enabled_flag {
            sps.pcm_bit_depth = bs.get_n::<u8>(4)? + 1;
            sps.pcm_bit_depth_chroma = bs.get_n::<u8>(4)? + 1;
            sps.pcm_log2_min_pcm_cb_size = bs.get_ue()? + 3;
            sps.pcm_log2_max_pcm_cb_size = bs.get_ue()? + sps.pcm_log2_min_pcm_cb_size;

            sps.pcm_loop_filter_disable_flag = bs.get()?;
        }

        sps.nb_st_rps = bs.get_ue()?;

        sps.short_term_ref_pic_sets
            .resize_with(sps.nb_st_rps as usize, Default::default);
        for i in 0..sps.nb_st_rps as usize {
            sps.short_term_ref_pic_sets[i] =
                ShortTermRPS::parse(bs, &sps, i, sps.nb_st_rps, false)?;
        }

        sps.long_term_ref_pics_present_flag = bs.get()?;

        if sps.long_term_ref_pics_present_flag {
            sps.num_long_term_ref_pics_sps = bs.get_ue()?;

            for _ in 0..sps.num_long_term_ref_pics_sps {
                sps.lt_ref_pic_poc_lsb_sps
                    .push(bs.get_n(sps.log2_max_poc_lsb as u32)?);
                sps.used_by_curr_pic_lt_sps_flag.push(bs.get()?);
            }
        }

        sps.sps_temporal_mvp_enabled_flag = bs.get()?;
        sps.sps_strong_intra_smoothing_enable_flag = bs.get()?;

        sps.vui_present = bs.get()?;

        if sps.vui_present {
            sps.vui_parameters = VuiParameters::parse(bs, sps.max_sub_layers)?;
        }

        sps.sps_extension_flag = bs.get()?;

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
