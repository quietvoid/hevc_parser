use std::{
    io::Write,
    path::{Path, PathBuf},
};

use anyhow::{bail, format_err, Result};
use regex::Regex;

pub mod processor;

use super::{hevc::*, HevcParser, NALUStartCode, NALUnit};

pub const FOUR_SIZED_NALU_TYPES: &[u8] = &[NAL_VPS, NAL_SPS, NAL_PPS, NAL_AUD, NAL_UNSPEC62];

#[derive(Debug, PartialEq, Clone, Eq)]
pub enum IoFormat {
    Raw,
    RawStdin,
    Matroska,
}

#[derive(Debug, Clone, Copy)]
pub enum StartCodePreset {
    Four,
    AnnexB,
}

pub trait IoProcessor {
    /// Input path
    fn input(&self) -> &PathBuf;
    /// If the processor has a progress bar, this updates every megabyte read
    fn update_progress(&mut self, delta: u64);

    /// NALU processing callback
    /// This is called after reading a 100kB chunk of the file
    /// The resulting NALs are always complete and unique
    ///
    /// The data can be access through `chunk`, using the NAL start/end indices
    fn process_nals(&mut self, parser: &HevcParser, nals: &[NALUnit], chunk: &[u8]) -> Result<()>;

    /// Finalize callback, when the stream is done being read
    /// Called at the end of `HevcProcessor::process_io`
    fn finalize(&mut self, parser: &HevcParser) -> Result<()>;
}

/// Data for a frame, with its decoded index
#[derive(Debug, Clone)]
pub struct FrameBuffer {
    pub frame_number: u64,
    pub nals: Vec<NalBuffer>,
}

/// Data for a NALU, with type
/// The data does not include the start code
#[derive(Debug, Clone)]
pub struct NalBuffer {
    pub nal_type: u8,
    pub start_code: NALUStartCode,
    pub data: Vec<u8>,
}

pub fn format_from_path(input: &Path) -> Result<IoFormat> {
    let regex = Regex::new(r"\.(hevc|.?265|mkv)")?;
    let file_name = match input.file_name() {
        Some(file_name) => file_name
            .to_str()
            .ok_or_else(|| format_err!("Invalid file name"))?,
        None => "",
    };

    if file_name == "-" {
        Ok(IoFormat::RawStdin)
    } else if regex.is_match(file_name) && input.is_file() {
        if file_name.ends_with(".mkv") {
            Ok(IoFormat::Matroska)
        } else {
            Ok(IoFormat::Raw)
        }
    } else if file_name.is_empty() {
        bail!("Missing input.")
    } else if !input.is_file() {
        bail!("Input file doesn't exist.")
    } else {
        bail!("Invalid input file type.")
    }
}

impl std::fmt::Display for IoFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            IoFormat::Matroska => write!(f, "Matroska file"),
            IoFormat::Raw => write!(f, "HEVC file"),
            IoFormat::RawStdin => write!(f, "HEVC pipe"),
        }
    }
}

impl NALUnit {
    pub fn write_with_preset(
        writer: &mut dyn Write,
        data: &[u8],
        preset: StartCodePreset,
        nal_type: u8,
        first_nal: bool,
    ) -> Result<()> {
        let start_code = match preset {
            StartCodePreset::Four => NALUStartCode::Length4,
            StartCodePreset::AnnexB => {
                if FOUR_SIZED_NALU_TYPES.contains(&nal_type) || first_nal {
                    NALUStartCode::Length4
                } else {
                    NALUStartCode::Length3
                }
            }
        };

        writer.write_all(start_code.slice())?;
        writer.write_all(data)?;

        Ok(())
    }
}
