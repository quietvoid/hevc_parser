use anyhow::{format_err, Result};

use super::BitVecReader;
use super::*;
use super::{pps::PPSNAL, sps::SPSNAL, NALUnit};

#[derive(Default, Debug, PartialEq, Clone, Eq)]
pub struct SliceNAL {
    pub first_slice_in_pic_flag: bool,
    pub key_frame: bool,
    pps_id: u64,
    pub slice_type: u64,

    dependent_slice_segment_flag: bool,
    slice_segment_addr: u64,

    pic_order_cnt_lsb: u64,
    pub output_picture_number: u64,
}

impl SliceNAL {
    pub fn parse(
        bs: &mut BitVecReader,
        sps_list: &[SPSNAL],
        pps_list: &[PPSNAL],
        nal: &NALUnit,
        poc_tid0: &mut u64,
        poc: &mut u64,
    ) -> Result<SliceNAL> {
        let mut slice = SliceNAL {
            first_slice_in_pic_flag: bs.get()?,
            ..Default::default()
        };

        if is_irap_nal(nal) {
            slice.key_frame = true;
            bs.skip_n(1); // no_output_of_prior_pics_flag
        }

        slice.pps_id = bs.get_ue()?;
        let pps = pps_list
            .get(slice.pps_id as usize)
            .ok_or_else(|| format_err!("Invalid PPS index"))?;
        let sps = sps_list
            .get(pps.sps_id as usize)
            .ok_or_else(|| format_err!("Invalid SPS index"))?;

        if !slice.first_slice_in_pic_flag {
            if pps.dependent_slice_segments_enabled_flag {
                slice.dependent_slice_segment_flag = bs.get()?;
            } else {
                slice.dependent_slice_segment_flag = false;
            }

            let pic_size = (sps.ctb_width * sps.ctb_height) as f64;
            let slice_address_length = pic_size.log2().ceil() as usize;

            slice.slice_segment_addr = bs.get_n(slice_address_length);
        } else {
            slice.dependent_slice_segment_flag = false;
        }

        if slice.dependent_slice_segment_flag {
            return Ok(slice);
        }

        for _ in 0..pps.num_extra_slice_header_bits {
            bs.skip_n(1); // slice_reserved_undetermined_flag
        }

        slice.slice_type = bs.get_ue()?;

        if pps.output_flag_present_flag {
            bs.skip_n(1);
        }

        if sps.separate_colour_plane_flag {
            bs.skip_n(2);
        }

        if !is_idr_nal(nal) {
            slice.pic_order_cnt_lsb = bs.get_n(sps.log2_max_poc_lsb as usize);
            slice.output_picture_number = compute_poc(sps, *poc_tid0, slice.pic_order_cnt_lsb, nal);
        } else {
            slice.output_picture_number = 0;
        }

        *poc = slice.output_picture_number;

        if nal.temporal_id == 0
            && nal.nal_type != NAL_TRAIL_N
            && nal.nal_type != NAL_TSA_N
            && nal.nal_type != NAL_STSA_N
            && nal.nal_type != NAL_RADL_N
            && nal.nal_type != NAL_RASL_N
            && nal.nal_type != NAL_RADL_R
            && nal.nal_type != NAL_RASL_R
        {
            *poc_tid0 = *poc;
        }

        Ok(slice)
    }
}

fn is_irap_nal(nal: &NALUnit) -> bool {
    nal.nal_type >= 16 && nal.nal_type <= 23
}

fn is_idr_nal(nal: &NALUnit) -> bool {
    nal.nal_type == NAL_IDR_W_RADL || nal.nal_type == NAL_IDR_N_LP
}

fn compute_poc(sps: &SPSNAL, poc_tid0: u64, poc_lsb: u64, nal: &NALUnit) -> u64 {
    let max_poc_lsb = 1 << sps.log2_max_poc_lsb;
    let prev_poc_lsb = poc_tid0 % max_poc_lsb;
    let prev_poc_msb = poc_tid0 - prev_poc_lsb;

    let mut poc_msb = if poc_lsb < prev_poc_lsb && prev_poc_lsb - poc_lsb >= max_poc_lsb / 2 {
        prev_poc_msb + max_poc_lsb
    } else if poc_lsb > prev_poc_lsb && poc_lsb - prev_poc_lsb > max_poc_lsb / 2 {
        prev_poc_msb - max_poc_lsb
    } else {
        prev_poc_msb
    };

    if nal.nal_type == NAL_BLA_W_LP
        || nal.nal_type == NAL_BLA_W_RADL
        || nal.nal_type == NAL_BLA_N_LP
    {
        poc_msb = 0;
    }

    poc_msb + poc_lsb
}
