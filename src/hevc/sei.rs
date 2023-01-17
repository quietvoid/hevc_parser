use super::{NAL_EOB_NUT, NAL_EOS_NUT, NAL_SEI_PREFIX, NAL_SEI_SUFFIX};
use anyhow::{bail, Result};
use bitvec_helpers::bitstream_io_reader::BsIoSliceReader;

#[derive(Default, Debug, Clone)]
pub struct SeiMessage {
    num_payload_type_ff_bytes: usize,
    last_payload_type_byte: u8,

    num_payload_size_ff_bytes: usize,
    last_payload_size_byte: u8,

    // Offset of the messame in the input slice
    pub msg_offset: usize,

    pub payload_type: u8,
    pub payload_offset: usize,
    pub payload_size: usize,
}

impl SeiMessage {
    /// Assumes the data does not contain any `emulation_prevention_three_byte`s
    pub fn parse_sei_rbsp(data: &[u8]) -> Result<Vec<SeiMessage>> {
        let mut reader = BsIoSliceReader::from_slice(data);

        // forbidden_zero_bit
        reader.skip_n(1)?;

        let nal_type = reader.get_n::<u8>(6)?;

        if nal_type != NAL_SEI_PREFIX && nal_type != NAL_SEI_SUFFIX {
            bail!("NAL type {} is not SEI", nal_type);
        }

        if reader.available()? < 9 && matches!(nal_type, NAL_EOS_NUT | NAL_EOB_NUT) {
        } else {
            reader.skip_n(6)?; // nuh_layer_id
            reader.skip_n(3)?; // temporal_id
        }

        let mut messages = Vec::new();

        loop {
            messages.push(Self::parse_sei_message(&mut reader)?);

            if reader.available()? <= 8 {
                break;
            }
        }

        Ok(messages)
    }

    fn parse_sei_message(reader: &mut BsIoSliceReader) -> Result<SeiMessage> {
        let mut msg = SeiMessage {
            msg_offset: (reader.position()? / 8) as usize,
            last_payload_type_byte: reader.get_n(8)?,
            ..Default::default()
        };

        while msg.last_payload_type_byte == 0xFF {
            msg.num_payload_type_ff_bytes += 1;
            msg.last_payload_type_byte = reader.get_n(8)?;

            msg.payload_type += 255;
        }

        msg.payload_type += msg.last_payload_type_byte;

        msg.last_payload_size_byte = reader.get_n(8)?;
        while msg.last_payload_size_byte == 0xFF {
            msg.num_payload_size_ff_bytes += 1;
            msg.last_payload_size_byte = reader.get_n(8)?;

            msg.payload_size += 255;
        }

        msg.payload_size += msg.last_payload_size_byte as usize;
        msg.payload_offset = (reader.position()? / 8) as usize;

        if msg.payload_size > reader.available()? as usize {
            bail!("Payload size is larger than NALU size");
        }

        reader.skip_n(msg.payload_size as u32 * 8)?;

        Ok(msg)
    }
}
