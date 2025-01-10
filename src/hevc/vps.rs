use anyhow::Result;

use super::hrd_parameters::HrdParameters;
use super::profile_tier_level::ProfileTierLevel;
use super::BsIoVecReader;

#[allow(clippy::upper_case_acronyms)]
#[derive(Default, Debug, PartialEq, Eq)]
pub struct VPSNAL {
    pub vps_id: u8,
    pub vps_max_layers: u8,
    pub vps_max_sub_layers: u8,
    pub vps_temporal_id_nesting_flag: bool,
    pub ptl: ProfileTierLevel,
    pub vps_sub_layer_ordering_info_present_flag: bool,
    pub vps_max_dec_pic_buffering: Vec<u64>,
    pub vps_num_reorder_pics: Vec<u64>,
    pub vps_max_latency_increase: Vec<u64>,
    pub vps_max_layer_id: u8,
    pub vps_num_layer_sets: u64,
    pub vps_timing_info_present_flag: bool,
    pub vps_num_units_in_tick: u32,
    pub vps_time_scale: u32,
    pub vps_poc_proportional_to_timing_flag: bool,
    pub vps_num_ticks_poc_diff_one: u64,
    pub vps_num_hrd_parameters: u64,
}

impl VPSNAL {
    pub fn parse(bs: &mut BsIoVecReader) -> Result<VPSNAL> {
        let mut vps = VPSNAL {
            vps_id: bs.get_n(4)?,
            ..Default::default()
        };

        // vps_reserved_three_2bits
        assert!(bs.get_n::<u8>(2)? == 3);

        vps.vps_max_layers = bs.get_n::<u8>(6)? + 1;
        vps.vps_max_sub_layers = bs.get_n::<u8>(3)? + 1;
        vps.vps_temporal_id_nesting_flag = bs.get()?;

        // vps_reserved_ffff_16bits
        assert!(bs.get_n::<u32>(16)? == 0xFFFF);

        vps.ptl.parse(bs, vps.vps_max_sub_layers)?;

        vps.vps_sub_layer_ordering_info_present_flag = bs.get()?;

        let i = if vps.vps_sub_layer_ordering_info_present_flag {
            0
        } else {
            vps.vps_max_sub_layers - 1
        };

        for _ in i..vps.vps_max_sub_layers {
            vps.vps_max_dec_pic_buffering.push(bs.get_ue()? + 1);
            vps.vps_num_reorder_pics.push(bs.get_ue()?);

            let mut vps_max_latency_increase = bs.get_ue()?;
            vps_max_latency_increase = vps_max_latency_increase.saturating_sub(1);

            vps.vps_max_latency_increase.push(vps_max_latency_increase);
        }

        vps.vps_max_layer_id = bs.get_n(6)?;
        vps.vps_num_layer_sets = bs.get_ue()? + 1;

        for _ in 1..vps.vps_num_layer_sets {
            for _ in 0..=vps.vps_max_layer_id {
                bs.skip_n(1)?; // layer_id_included_flag[i][j]
            }
        }

        vps.vps_timing_info_present_flag = bs.get()?;

        if vps.vps_timing_info_present_flag {
            vps.vps_num_units_in_tick = bs.get_n(32)?;
            vps.vps_time_scale = bs.get_n(32)?;
            vps.vps_poc_proportional_to_timing_flag = bs.get()?;

            if vps.vps_poc_proportional_to_timing_flag {
                vps.vps_num_ticks_poc_diff_one = bs.get_ue()? + 1;
            }

            vps.vps_num_hrd_parameters = bs.get_ue()?;

            for i in 0..vps.vps_num_hrd_parameters {
                let mut common_inf_present = false;
                bs.get_ue()?; // hrd_layer_set_idx

                if i > 0 {
                    common_inf_present = bs.get()?;
                }

                HrdParameters::parse(bs, common_inf_present, vps.vps_max_sub_layers)?;
            }
        }

        bs.skip_n(1)?; // vps_extension_flag

        Ok(vps)
    }
}
