use crate::{vps::VPSNAL, sps::SPSNAL, pps::PPSNAL};

/// Contextual data that needs to be tracked between evaluations of different portions of H265
/// syntax.
pub struct Context {
    vid_param_sets: Vec<Option<VPSNAL>>,
    seq_param_sets: Vec<Option<SPSNAL>>,
    pic_param_sets: Vec<Option<PPSNAL>>,
}
impl Default for Context {
    fn default() -> Self {
        Self::new()
    }
}
impl Context {
    pub fn new() -> Self {
        let mut vid_param_sets = vec!();
        for _ in 0..16 { vid_param_sets.push(None); }

        let mut seq_param_sets = vec!();
        for _ in 0..16 { seq_param_sets.push(None); }

        let mut pic_param_sets = vec!();
        for _ in 0..64 { pic_param_sets.push(None); }

        Context {
            vid_param_sets,
            seq_param_sets,
            pic_param_sets,
        }
    }
}
impl Context {
    pub fn vps_by_id(&self, id: u8) -> Option<&VPSNAL> {
        if id > 15 {
            None
        } else {
            self.vid_param_sets[id as usize].as_ref()
        }
    }
    pub fn vps(&self) -> impl Iterator<Item = &VPSNAL> {
        self.vid_param_sets.iter().filter_map(Option::as_ref)
    }
    pub fn put_vid_param_set(&mut self, vps: VPSNAL) {
        let i = vps.vps_id as usize;
        self.vid_param_sets[i] = Some(vps);
    }
    pub fn sps_by_id(&self, id: u64) -> Option<&SPSNAL> {
        if id > 15 {
            None
        } else {
            self.seq_param_sets[id as usize].as_ref()
        }
    }
    pub fn sps(&self) -> impl Iterator<Item = &SPSNAL> {
        self.seq_param_sets.iter().filter_map(Option::as_ref)
    }
    pub fn put_seq_param_set(&mut self, sps: SPSNAL) {
        let i = sps.sps_id as usize;
        self.seq_param_sets[i] = Some(sps);
    }
    pub fn pps_by_id(&self, id: u64) -> Option<&PPSNAL> {
        if id > 63 {
            None
        } else {
            self.pic_param_sets[id as usize].as_ref()
        }
    }
    pub fn pps(&self) -> impl Iterator<Item = &PPSNAL> {
        self.pic_param_sets.iter().filter_map(Option::as_ref)
    }
    pub fn put_pic_param_set(&mut self, pps: PPSNAL) {
        let i = pps.pps_id as usize;
        self.pic_param_sets[i] = Some(pps);
    }
}
