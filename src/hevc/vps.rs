use anyhow::Result;

use super::BsIoVecReader;
use super::hrd_parameters::HrdParameters;
use super::profile_tier_level::ProfileTierLevel;

#[allow(clippy::upper_case_acronyms)]
#[derive(Default, Debug, PartialEq, Eq)]
pub struct VPSNAL {
    pub(crate) vps_id: u8,
    vps_max_layers: u8,
    vps_max_sub_layers: u8,
    vps_temporal_id_nesting_flag: bool,
    ptl: ProfileTierLevel,
    vps_sub_layer_ordering_info_present_flag: bool,
    vps_max_dec_pic_buffering: Vec<u64>,
    vps_num_reorder_pics: Vec<u64>,
    vps_max_latency_increase: Vec<u64>,
    vps_max_layer_id: u8,
    vps_num_layer_sets: u64,
    vps_timing_info_present_flag: bool,
    vps_num_units_in_tick: u32,
    vps_time_scale: u32,
    vps_poc_proportional_to_timing_flag: bool,
    vps_num_ticks_poc_diff_one: u64,
    vps_num_hrd_parameters: u64,
}

impl VPSNAL {
    pub fn parse(bs: &mut BsIoVecReader) -> Result<VPSNAL> {
        let mut vps = VPSNAL {
            vps_id: bs.read::<4, u8>()?,
            ..Default::default()
        };

        // vps_reserved_three_2bits
        assert!(bs.read::<2, u8>()? == 3);

        vps.vps_max_layers = bs.read::<6, u8>()? + 1;
        vps.vps_max_sub_layers = bs.read::<3, u8>()? + 1;
        vps.vps_temporal_id_nesting_flag = bs.read_bit()?;

        // vps_reserved_ffff_16bits
        assert!(bs.read::<16, u16>()? == 0xFFFF);

        vps.ptl.parse(bs, vps.vps_max_sub_layers)?;

        vps.vps_sub_layer_ordering_info_present_flag = bs.read_bit()?;

        let i = if vps.vps_sub_layer_ordering_info_present_flag {
            0
        } else {
            vps.vps_max_sub_layers - 1
        };

        for _ in i..vps.vps_max_sub_layers {
            vps.vps_max_dec_pic_buffering.push(bs.read_ue()? + 1);
            vps.vps_num_reorder_pics.push(bs.read_ue()?);

            let mut vps_max_latency_increase = bs.read_ue()?;
            vps_max_latency_increase = vps_max_latency_increase.saturating_sub(1);

            vps.vps_max_latency_increase.push(vps_max_latency_increase);
        }

        vps.vps_max_layer_id = bs.read::<6, u8>()?;
        vps.vps_num_layer_sets = bs.read_ue()? + 1;

        for _ in 1..vps.vps_num_layer_sets {
            for _ in 0..=vps.vps_max_layer_id {
                bs.skip_n(1)?; // layer_id_included_flag[i][j]
            }
        }

        vps.vps_timing_info_present_flag = bs.read_bit()?;

        if vps.vps_timing_info_present_flag {
            vps.vps_num_units_in_tick = bs.read::<32, u32>()?;
            vps.vps_time_scale = bs.read::<32, u32>()?;
            vps.vps_poc_proportional_to_timing_flag = bs.read_bit()?;

            if vps.vps_poc_proportional_to_timing_flag {
                vps.vps_num_ticks_poc_diff_one = bs.read_ue()? + 1;
            }

            vps.vps_num_hrd_parameters = bs.read_ue()?;

            for i in 0..vps.vps_num_hrd_parameters {
                let mut common_inf_present = false;
                bs.read_ue()?; // hrd_layer_set_idx

                if i > 0 {
                    common_inf_present = bs.read_bit()?;
                }

                HrdParameters::parse(bs, common_inf_present, vps.vps_max_sub_layers)?;
            }
        }

        bs.skip_n(1)?; // vps_extension_flag

        Ok(vps)
    }
}
