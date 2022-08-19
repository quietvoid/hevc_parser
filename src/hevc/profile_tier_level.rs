use super::BitVecReader;
use anyhow::Result;

#[derive(Default, Debug, PartialEq, Clone, Eq)]
pub struct ProfileTierLevel {
    pub general_profile_space: u8,
    pub general_tier_flag: bool,
    pub general_profile_idc: u8,
    pub general_profile_compatibility_flag: Vec<bool>,
    pub general_progressive_source_flag: bool,
    pub general_interlaced_source_flag: bool,
    pub general_non_packed_constraint_flag: bool,
    pub general_frame_only_constraint_flag: bool,
    pub general_level_idc: u8,

    pub sub_layer_profile_present_flag: Vec<bool>,
    pub sub_layer_level_present_flag: Vec<bool>,
    pub sub_layer_profile_space: Vec<u8>,
    pub sub_layer_tier_flag: Vec<bool>,
    pub sub_layer_profile_idc: Vec<u8>,
    pub sub_layer_profile_compatibility_flag: Vec<bool>,
    pub sub_layer_progressive_source_flag: Vec<bool>,
    pub sub_layer_interlaced_source_flag: Vec<bool>,
    pub sub_layer_non_packed_constraint_flag: Vec<bool>,
    pub sub_layer_frame_only_constraint_flag: Vec<bool>,
    pub sub_layer_level_idc: Vec<u8>,
}

impl ProfileTierLevel {
    pub fn parse(&mut self, bs: &mut BitVecReader, max_sub_layers: u8) -> Result<()> {
        self.general_profile_space = bs.get_n(2);
        self.general_tier_flag = bs.get()?;
        self.general_profile_idc = bs.get_n(5);

        for _ in 0..32 {
            self.general_profile_compatibility_flag.push(bs.get()?);
        }

        self.general_progressive_source_flag = bs.get()?;
        self.general_interlaced_source_flag = bs.get()?;
        self.general_non_packed_constraint_flag = bs.get()?;
        self.general_frame_only_constraint_flag = bs.get()?;
        bs.skip_n(32);
        bs.skip_n(12);
        self.general_level_idc = bs.get_n(8);

        let max_sub_layers_minus1 = max_sub_layers - 1;
        for _ in 0..max_sub_layers_minus1 {
            self.sub_layer_profile_present_flag.push(bs.get()?);
            self.sub_layer_level_present_flag.push(bs.get()?);
        }

        if max_sub_layers_minus1 > 0 {
            for _ in max_sub_layers_minus1..8 {
                bs.skip_n(2);
            }
        }

        for i in 0..max_sub_layers_minus1 as usize {
            if self.sub_layer_profile_present_flag[i] {
                self.sub_layer_profile_space.push(bs.get_n(2));
                self.sub_layer_tier_flag.push(bs.get()?);
                self.sub_layer_profile_idc.push(bs.get_n(5));

                for _ in 0..32 {
                    self.sub_layer_profile_compatibility_flag.push(bs.get()?);
                }

                self.sub_layer_progressive_source_flag.push(bs.get()?);
                self.sub_layer_interlaced_source_flag.push(bs.get()?);
                self.sub_layer_non_packed_constraint_flag.push(bs.get()?);
                self.sub_layer_frame_only_constraint_flag.push(bs.get()?);

                bs.skip_n(32);
                bs.skip_n(12);
            }

            if self.sub_layer_level_present_flag[i] {
                self.sub_layer_level_idc.push(bs.get_n(8));
            } else {
                self.sub_layer_level_idc.push(1);
            }
        }

        Ok(())
    }
}
