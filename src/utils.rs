use super::{Frame, NALUStartCode, NAL_AUD};
use bitvec_helpers::bitvec_writer::BitVecWriter;

pub fn clear_start_code_emulation_prevention_3_byte(data: &[u8]) -> Vec<u8> {
    data.iter()
        .enumerate()
        .filter_map(|(index, value)| {
            if index > 2
                && index < data.len() - 2
                && data[index - 2] == 0
                && data[index - 1] == 0
                && data[index] == 3
            {
                None
            } else {
                Some(*value)
            }
        })
        .collect::<Vec<u8>>()
}

/// Within the NAL unit, the following three-byte sequences shall not occur at any byte-aligned position:
///   - 0x000000
///   - 0x000001
///   - 0x000002
pub fn add_start_code_emulation_prevention_3_byte(data: &mut Vec<u8>) {
    let mut count = data.len();
    let mut i = 0;

    while i < count {
        if i > 2 && i < count - 2 && data[i - 2] == 0 && data[i - 1] == 0 && data[i] <= 3 {
            data.insert(i, 3);
            count += 1;
        }

        i += 1;
    }
}

pub fn aud_for_frame(frame: &Frame, start_code: Option<NALUStartCode>) -> Vec<u8> {
    let pic_type: u8 = match &frame.frame_type {
        2 => 0, // I
        1 => 1, // P, I
        0 => 2, // B, P, I
        _ => 7,
    };

    let mut data = if let Some(sc) = start_code {
        sc.slice().to_vec()
    } else {
        Vec::new()
    };

    let mut writer = BitVecWriter::new();

    writer.write(false); // forbidden_zero_bit

    writer.write_n(&(NAL_AUD).to_be_bytes(), 6); // nal_unit_type
    writer.write_n(&(0_u8).to_be_bytes(), 6); // nuh_layer_id
    writer.write_n(&(1_u8).to_be_bytes(), 3); // nuh_temporal_id_plus1

    writer.write_n(&pic_type.to_be_bytes(), 3); // pic_type

    // rbsp_trailing_bits()
    writer.write(true); // rbsp_stop_one_bit

    while !writer.is_aligned() {
        writer.write(false); // rbsp_alignment_zero_bit
    }

    data.extend_from_slice(writer.as_slice());

    data
}
