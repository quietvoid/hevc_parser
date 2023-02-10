use anyhow::{anyhow, Result};
use bitvec_helpers::bitstream_io_writer::BitstreamIoWriter;

use super::{Frame, NALUStartCode, NAL_AUD};

pub fn clear_start_code_emulation_prevention_3_byte(data: &[u8]) -> Vec<u8> {
    let len = data.len();

    if len > 2 {
        let mut unescaped_bytes: Vec<u8> = Vec::with_capacity(len);
        unescaped_bytes.push(data[0]);
        unescaped_bytes.push(data[1]);

        for i in 2..len {
            if !(data[i - 2] == 0 && data[i - 1] == 0 && data[i] == 3) {
                unescaped_bytes.push(data[i]);
            }
        }

        unescaped_bytes
    } else {
        data.to_owned()
    }
}

/// Within the NAL unit, the following three-byte sequences shall not occur at any byte-aligned position:
///   - 0x000000
///   - 0x000001
///   - 0x000002
pub fn add_start_code_emulation_prevention_3_byte(data: &mut Vec<u8>) {
    let mut count = data.len();
    let mut i = 0;

    while i < count {
        if i > 2 && data[i - 2] == 0 && data[i - 1] == 0 && data[i] <= 3 {
            data.insert(i, 3);
            count += 1;
        }

        i += 1;
    }
}

pub fn aud_for_frame(frame: &Frame, start_code: Option<NALUStartCode>) -> Result<Vec<u8>> {
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

    let mut writer = BitstreamIoWriter::with_capacity(24);

    writer.write(false)?; // forbidden_zero_bit

    writer.write_n(&NAL_AUD, 6)?; // nal_unit_type
    writer.write_n(&0_u8, 6)?; // nuh_layer_id
    writer.write_n(&1_u8, 3)?; // nuh_temporal_id_plus1

    writer.write_n(&pic_type, 3)?; // pic_type

    // rbsp_trailing_bits()
    writer.write(true)?; // rbsp_stop_one_bit

    // rbsp_alignment_zero_bit
    writer.byte_align()?;

    data.extend_from_slice(
        writer
            .as_slice()
            .ok_or_else(|| anyhow!("Unaligned bytes"))?,
    );

    Ok(data)
}
