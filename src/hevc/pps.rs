use super::{scaling_list_data::ScalingListData, BitVecReader};
use anyhow::Result;

#[allow(clippy::upper_case_acronyms)]
#[derive(Default, Debug, PartialEq, Eq)]
pub struct PPSNAL {
    pub(crate) pps_id: u64,
    pub(crate) sps_id: u64,
    pub(crate) dependent_slice_segments_enabled_flag: bool,
    pub(crate) output_flag_present_flag: bool,
    pub(crate) num_extra_slice_header_bits: u8,
    sign_data_hiding_flag: bool,
    cabac_init_present_flag: bool,
    num_ref_idx_l0_default_active: u64,
    num_ref_idx_l1_default_active: u64,
    pic_init_qp_minus26: i64,
    constrained_intra_pred_flag: bool,
    transform_skip_enabled_flag: bool,
    cu_qp_delta_enabled_flag: bool,
    diff_cu_qp_delta_depth: u64,
    cb_qp_offset: i64,
    cr_qp_offset: i64,
    pic_slice_level_chroma_qp_offsets_present_flag: bool,
    weighted_pred_flag: bool,
    weighted_bipred_flag: bool,
    transquant_bypass_enable_flag: bool,
    tiles_enabled_flag: bool,
    entropy_coding_sync_enabled_flag: bool,

    num_tile_columns: u64,
    num_tile_rows: u64,
    uniform_spacing_flag: bool,

    column_widths: Vec<u64>,
    row_heights: Vec<u64>,

    loop_filter_across_tiles_enabled_flag: bool,
    seq_loop_filter_across_slices_enabled_flag: bool,
    deblocking_filter_control_present_flag: bool,
    deblocking_filter_override_enabled_flag: bool,
    disable_dbf: bool,
    beta_offset: i64,
    tc_offset: i64,

    scaling_list_data_present_flag: bool,
    scaling_list_data: ScalingListData,

    lists_modification_present_flag: bool,
    log2_parallel_merge_level: u64,
    slice_header_extension_present_flag: bool,
    pps_extension_present_flag: bool,
}

impl PPSNAL {
    pub fn parse(bs: &mut BitVecReader) -> Result<PPSNAL> {
        let mut pps = PPSNAL {
            pps_id: bs.get_ue()?,
            sps_id: bs.get_ue()?,
            ..Default::default()
        };

        pps.dependent_slice_segments_enabled_flag = bs.get()?;
        pps.output_flag_present_flag = bs.get()?;
        pps.num_extra_slice_header_bits = bs.get_n(3);
        pps.sign_data_hiding_flag = bs.get()?;
        pps.cabac_init_present_flag = bs.get()?;
        pps.num_ref_idx_l0_default_active = bs.get_ue()? + 1;
        pps.num_ref_idx_l1_default_active = bs.get_ue()? + 1;
        pps.pic_init_qp_minus26 = bs.get_se()?;
        pps.constrained_intra_pred_flag = bs.get()?;
        pps.transform_skip_enabled_flag = bs.get()?;
        pps.cu_qp_delta_enabled_flag = bs.get()?;

        pps.diff_cu_qp_delta_depth = if pps.cu_qp_delta_enabled_flag {
            bs.get_ue()?
        } else {
            0
        };

        pps.cb_qp_offset = bs.get_se()?;
        pps.cr_qp_offset = bs.get_se()?;

        pps.pic_slice_level_chroma_qp_offsets_present_flag = bs.get()?;
        pps.weighted_pred_flag = bs.get()?;
        pps.weighted_bipred_flag = bs.get()?;

        pps.transquant_bypass_enable_flag = bs.get()?;
        pps.tiles_enabled_flag = bs.get()?;
        pps.entropy_coding_sync_enabled_flag = bs.get()?;

        if pps.tiles_enabled_flag {
            pps.num_tile_columns = bs.get_ue()? + 1;
            pps.num_tile_rows = bs.get_ue()? + 1;

            pps.uniform_spacing_flag = bs.get()?;

            if !pps.uniform_spacing_flag {
                for _ in 0..pps.num_tile_columns - 1 {
                    pps.column_widths.push(bs.get_ue()? + 1);
                }

                for _ in 0..pps.num_tile_rows - 1 {
                    pps.row_heights.push(bs.get_ue()? + 1);
                }
            }

            pps.loop_filter_across_tiles_enabled_flag = bs.get()?;
        }

        pps.seq_loop_filter_across_slices_enabled_flag = bs.get()?;
        pps.deblocking_filter_control_present_flag = bs.get()?;

        if pps.deblocking_filter_control_present_flag {
            pps.deblocking_filter_override_enabled_flag = bs.get()?;
            pps.disable_dbf = bs.get()?;

            if !pps.disable_dbf {
                pps.beta_offset = 2 * bs.get_se()?;
                pps.tc_offset = 2 * bs.get_se()?;
            }
        }

        pps.scaling_list_data_present_flag = bs.get()?;
        if pps.scaling_list_data_present_flag {
            pps.scaling_list_data = ScalingListData::parse(bs)?;
        }

        pps.lists_modification_present_flag = bs.get()?;
        pps.log2_parallel_merge_level = bs.get_ue()? + 2;

        pps.slice_header_extension_present_flag = bs.get()?;
        pps.pps_extension_present_flag = bs.get()?;

        Ok(pps)
    }
}
