use anyhow::Result;

use super::BsIoVecReader;
use super::sps::SPSNAL;

#[derive(Default, Debug, PartialEq, Clone, Eq)]
pub struct ShortTermRPS {
    inter_ref_pic_set_prediction_flag: bool,
    delta_idx: u64,
    delta_rps_sign: bool,
    abs_delta_rps: u64,
    used_by_curr_pic_flags: Vec<bool>,
    use_delta_flags: Vec<bool>,
    num_delta_pocs: u64,
    num_negative_pics: u64,
    num_positive_pics: u64,

    delta_poc_s0: Vec<u64>,
    used_by_curr_pic_s0_flags: Vec<bool>,
    delta_poc_s1: Vec<u64>,
    used_by_curr_pic_s1_flags: Vec<bool>,
}

impl ShortTermRPS {
    pub fn parse(
        bs: &mut BsIoVecReader,
        sps: &SPSNAL,
        st_rps_idx: usize,
        nb_st_rps: u64,
        is_slice_header: bool,
    ) -> Result<ShortTermRPS> {
        let mut rps = ShortTermRPS::default();

        if st_rps_idx > 0 && nb_st_rps > 0 {
            rps.inter_ref_pic_set_prediction_flag = bs.get()?;
        }

        if rps.inter_ref_pic_set_prediction_flag {
            let ref_pic_sets = &sps.short_term_ref_pic_sets;

            if st_rps_idx == nb_st_rps as usize || is_slice_header {
                rps.delta_idx = bs.get_ue()?;
            }

            rps.delta_rps_sign = bs.get()?;
            rps.abs_delta_rps = bs.get_ue()? + 1;

            let ref_rps_idx = st_rps_idx - (rps.delta_idx as usize + 1);
            let mut num_delta_pocs: usize = 0;
            let ref_rps = &ref_pic_sets[ref_rps_idx];

            if ref_rps.inter_ref_pic_set_prediction_flag {
                for i in 0..ref_rps.used_by_curr_pic_flags.len() {
                    if ref_rps.used_by_curr_pic_flags[i] || ref_rps.use_delta_flags[i] {
                        num_delta_pocs += 1;
                    }
                }
            } else {
                num_delta_pocs = (ref_rps.num_negative_pics + ref_rps.num_positive_pics) as usize;
            }

            rps.used_by_curr_pic_flags.resize(num_delta_pocs + 1, false);
            rps.use_delta_flags.resize(num_delta_pocs + 1, true);

            for i in 0..=num_delta_pocs {
                rps.used_by_curr_pic_flags[i] = bs.get()?;

                if !rps.used_by_curr_pic_flags[i] {
                    rps.use_delta_flags[i] = bs.get()?;
                }
            }
        } else {
            rps.num_negative_pics = bs.get_ue()?;
            rps.num_positive_pics = bs.get_ue()?;

            for _ in 0..rps.num_negative_pics {
                rps.delta_poc_s0.push(bs.get_ue()? + 1);
                rps.used_by_curr_pic_s0_flags.push(bs.get()?);
            }

            for _ in 0..rps.num_positive_pics {
                rps.delta_poc_s1.push(bs.get_ue()? + 1);
                rps.used_by_curr_pic_s1_flags.push(bs.get()?);
            }
        }

        Ok(rps)
    }
}
