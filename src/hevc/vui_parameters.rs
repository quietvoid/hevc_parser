use anyhow::Result;

use super::hrd_parameters::HrdParameters;
use super::BsIoVecReader;

#[derive(Default, Debug, PartialEq, Clone, Eq)]
pub struct VuiParameters {
    pub sar_present: bool,
    pub sar_idc: u8,
    pub sar_num: u16,
    pub sar_den: u16,
    pub overscan_info_present_flag: bool,
    pub overscan_appropriate_flag: bool,
    pub video_signal_type_present_flag: bool,

    pub video_format: u8,
    pub video_full_range_flag: bool,
    pub colour_description_present_flag: bool,
    pub colour_primaries: u8,
    pub transfer_characteristic: u8,
    pub matrix_coeffs: u8,

    pub chroma_loc_info_present_flag: bool,
    pub chroma_sample_loc_type_top_field: u64,
    pub chroma_sample_loc_type_bottom_field: u64,
    pub neutral_chroma_indication_flag: bool,
    pub field_seq_flag: bool,
    pub frame_field_info_present_flag: bool,

    pub default_display_window_flag: bool,
    pub def_disp_win_left_offset: u64,
    pub def_disp_win_right_offset: u64,
    pub def_disp_win_top_offset: u64,
    pub def_disp_win_bottom_offset: u64,

    pub vui_timing_info_present_flag: bool,
    pub vui_num_units_in_tick: u32,
    pub vui_time_scale: u32,
    pub vui_poc_proportional_to_timing_flag: bool,
    pub vui_num_ticks_poc_diff_one_minus1: u64,
    pub vui_hrd_parameters_present_flag: bool,

    pub bitstream_restriction_flag: bool,
    pub tiles_fixed_structure_flag: bool,
    pub motion_vectors_over_pic_boundaries_flag: bool,
    pub restricted_ref_pic_lists_flag: bool,

    pub min_spatial_segmentation_idc: u64,
    pub max_bytes_per_pic_denom: u64,
    pub max_bits_per_min_cu_denom: u64,
    pub log2_max_mv_length_horizontal: u64,
    pub log2_max_mv_length_vertical: u64,
}

impl VuiParameters {
    pub fn parse(bs: &mut BsIoVecReader, max_sub_layers: u8) -> Result<VuiParameters> {
        let mut vui = VuiParameters {
            sar_present: bs.get()?,
            ..Default::default()
        };

        if vui.sar_present {
            vui.sar_idc = bs.get_n(8)?;

            if vui.sar_idc == 255 {
                vui.sar_num = bs.get_n(16)?;
                vui.sar_den = bs.get_n(16)?;
            }
        }

        vui.overscan_info_present_flag = bs.get()?;
        if vui.overscan_info_present_flag {
            vui.overscan_appropriate_flag = bs.get()?;
        }

        vui.video_signal_type_present_flag = bs.get()?;
        if vui.video_signal_type_present_flag {
            vui.video_format = bs.get_n(3)?;
            vui.video_full_range_flag = bs.get()?;
            vui.colour_description_present_flag = bs.get()?;

            if vui.colour_description_present_flag {
                vui.colour_primaries = bs.get_n(8)?;
                vui.transfer_characteristic = bs.get_n(8)?;
                vui.matrix_coeffs = bs.get_n(8)?;
            }
        }

        vui.chroma_loc_info_present_flag = bs.get()?;
        if vui.chroma_loc_info_present_flag {
            vui.chroma_sample_loc_type_top_field = bs.get_ue()?;
            vui.chroma_sample_loc_type_bottom_field = bs.get_ue()?;
        }

        vui.neutral_chroma_indication_flag = bs.get()?;
        vui.field_seq_flag = bs.get()?;
        vui.frame_field_info_present_flag = bs.get()?;
        vui.default_display_window_flag = bs.get()?;

        if vui.default_display_window_flag {
            vui.def_disp_win_left_offset = bs.get_ue()?;
            vui.def_disp_win_right_offset = bs.get_ue()?;
            vui.def_disp_win_top_offset = bs.get_ue()?;
            vui.def_disp_win_bottom_offset = bs.get_ue()?;
        }

        vui.vui_timing_info_present_flag = bs.get()?;
        if vui.vui_timing_info_present_flag {
            vui.vui_num_units_in_tick = bs.get_n(32)?;
            vui.vui_time_scale = bs.get_n(32)?;

            vui.vui_poc_proportional_to_timing_flag = bs.get()?;
            if vui.vui_poc_proportional_to_timing_flag {
                vui.vui_num_ticks_poc_diff_one_minus1 = bs.get_ue()?;
            }

            vui.vui_hrd_parameters_present_flag = bs.get()?;
            if vui.vui_hrd_parameters_present_flag {
                HrdParameters::parse(bs, true, max_sub_layers)?;
            }
        }

        vui.bitstream_restriction_flag = bs.get()?;
        if vui.bitstream_restriction_flag {
            vui.tiles_fixed_structure_flag = bs.get()?;
            vui.motion_vectors_over_pic_boundaries_flag = bs.get()?;
            vui.restricted_ref_pic_lists_flag = bs.get()?;

            vui.min_spatial_segmentation_idc = bs.get_ue()?;
            vui.max_bytes_per_pic_denom = bs.get_ue()?;
            vui.max_bits_per_min_cu_denom = bs.get_ue()?;
            vui.log2_max_mv_length_horizontal = bs.get_ue()?;
            vui.log2_max_mv_length_vertical = bs.get_ue()?;
        }

        Ok(vui)
    }

    pub fn aspect_ratio(&self) -> Option<(u16, u16)> {
        if !self.sar_present {
            return None;
        }

        match self.sar_idc {
            1 => Some((1, 1)),
            2 => Some((12, 11)),
            3 => Some((10, 11)),
            4 => Some((16, 11)),
            5 => Some((40, 33)),
            6 => Some((24, 11)),
            7 => Some((20, 11)),
            8 => Some((32, 11)),
            9 => Some((80, 33)),
            10 => Some((18, 11)),
            11 => Some((15, 11)),
            12 => Some((64, 33)),
            13 => Some((160, 99)),
            14 => Some((4, 3)),
            15 => Some((3, 2)),
            16 => Some((2, 1)),
            255 => Some((self.sar_num, self.sar_den)),
            _ => None,
        }
    }
}
