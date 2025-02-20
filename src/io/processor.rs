use std::{
    fs::File,
    io::{BufRead, BufReader, Read},
    path::Path,
};

use anyhow::{Result, bail, ensure};
use bitvec_helpers::bitstream_io_reader::BsIoSliceReader;
use matroska_demuxer::{MatroskaFile, TrackType};

use crate::{MAX_PARSE_SIZE, NALUStartCode, config::HEVCDecoderConfigurationRecord, hevc::NALUnit};

use super::{HevcParser, IoFormat, IoProcessor};

/// Base HEVC stream processor
pub struct HevcProcessor {
    opts: HevcProcessorOpts,

    format: IoFormat,
    parser: HevcParser,

    chunk_size: usize,

    main_buf: Vec<u8>,
    sec_buf: Vec<u8>,
    consumed: usize,

    chunk: Vec<u8>,
    end: Vec<u8>,
    offsets: Vec<usize>,

    last_buffered_frame: u64,
}

/// Options for the processor
pub struct HevcProcessorOpts {
    /// Buffer a frame when using `parse_nalus`.
    /// This stops the stream reading as soon as a full frame has been parsed.
    pub buffer_frame: bool,
    /// Parse the NALs, required for `buffer_frame`
    /// Provides frame presentation order
    pub parse_nals: bool,

    /// Stop reading the stream after N frames
    /// The number of processed frames may differ.
    pub limit: Option<u64>,
}

impl HevcProcessor {
    /// Initialize a HEVC stream processor
    pub fn new(format: IoFormat, opts: HevcProcessorOpts, chunk_size: usize) -> Self {
        let sec_buf = if format == IoFormat::RawStdin {
            vec![0; 50_000]
        } else {
            Vec::new()
        };

        Self {
            opts,
            format,
            parser: HevcParser::default(),

            chunk_size,
            main_buf: vec![0; chunk_size],
            sec_buf,
            consumed: 0,

            chunk: Vec::with_capacity(chunk_size),
            end: Vec::with_capacity(chunk_size),
            offsets: Vec::with_capacity(2048),

            last_buffered_frame: 0,
        }
    }

    /// Fully parse the input stream
    pub fn process_io(
        &mut self,
        reader: &mut dyn Read,
        processor: &mut dyn IoProcessor,
    ) -> Result<()> {
        self.parse_nalus(reader, processor)?;

        self.parser.finish();

        processor.finalize(&self.parser)?;

        Ok(())
    }

    /// Fully parse a file or input stream.  
    /// If `file_path` is `None`, the format must be `RawStdin`.
    pub fn process_file<P: AsRef<Path>>(
        &mut self,
        processor: &mut dyn IoProcessor,
        file_path: Option<P>,
    ) -> Result<()> {
        if let Some(input) = file_path {
            let file = File::open(input)?;

            match self.format {
                IoFormat::Matroska => self.process_matroska_file(processor, file),
                IoFormat::Raw => {
                    let mut reader = Box::new(BufReader::with_capacity(100_000, file));
                    self.process_io(&mut reader, processor)
                }
                _ => unreachable!(),
            }
        } else if let IoFormat::RawStdin = self.format {
            let stdin = std::io::stdin();
            let mut reader = Box::new(stdin.lock()) as Box<dyn BufRead>;

            self.process_io(&mut reader, processor)
        } else {
            bail!("Invalid params");
        }
    }

    /// Parse NALUs from the stream
    /// Depending on the options, this either:
    ///   - Loops the entire stream until EOF
    ///   - Loops until a complete frame has been parsed
    ///
    /// In both cases, the processor callback is called when a NALU payload is ready.
    pub fn parse_nalus(
        &mut self,
        reader: &mut dyn Read,
        processor: &mut dyn IoProcessor,
    ) -> Result<()> {
        while let Ok(n) = reader.read(&mut self.main_buf) {
            let mut read_bytes = n;
            if read_bytes == 0 && self.end.is_empty() && self.chunk.is_empty() {
                break;
            }

            if self.format == IoFormat::RawStdin {
                self.chunk.extend_from_slice(&self.main_buf[..read_bytes]);

                loop {
                    let num = reader.read(&mut self.sec_buf)?;
                    if num > 0 {
                        read_bytes += num;

                        self.chunk.extend_from_slice(&self.sec_buf[..num]);

                        if read_bytes >= self.chunk_size {
                            break;
                        }
                    } else {
                        break;
                    }
                }
            } else if read_bytes < self.chunk_size {
                self.chunk.extend_from_slice(&self.main_buf[..read_bytes]);
            } else {
                self.chunk.extend_from_slice(&self.main_buf);
            }

            self.parser.get_offsets(&self.chunk, &mut self.offsets);

            if self.offsets.is_empty() {
                if read_bytes == 0 {
                    break;
                }
                continue;
            }

            let last = if read_bytes < self.chunk_size {
                *self.offsets.last().unwrap()
            } else {
                let last = self.offsets.pop().unwrap();

                self.end.clear();
                self.end.extend_from_slice(&self.chunk[last..]);

                last
            };

            let nals =
                self.parser
                    .split_nals(&self.chunk, &self.offsets, last, self.opts.parse_nals)?;

            // Process NALUs
            processor.process_nals(&self.parser, &nals, &self.chunk)?;

            self.chunk.clear();

            if !self.end.is_empty() {
                self.chunk.extend_from_slice(&self.end);
                self.end.clear()
            }

            self.consumed += read_bytes;

            if self.consumed >= 100_000_000 {
                processor.update_progress(1);
                self.consumed = 0;
            }

            let check_current_frame_idx = self.opts.buffer_frame || self.opts.limit.is_some();
            if check_current_frame_idx {
                let max_frame_idx = nals.iter().map(|nal| nal.decoded_frame_index).max();

                if let Some(frame_idx) = max_frame_idx {
                    if self.opts.limit.is_some_and(|limit| frame_idx > limit) {
                        // Must be higher than limit, to make sure that the AU was fully read
                        break;
                    }

                    if self.opts.buffer_frame && frame_idx > self.last_buffered_frame {
                        self.last_buffered_frame = frame_idx;
                        break;
                    }
                }
            }
        }

        Ok(())
    }

    fn process_matroska_file(&mut self, processor: &mut dyn IoProcessor, file: File) -> Result<()> {
        let mut mkv = MatroskaFile::open(file)?;
        let track = mkv
            .tracks()
            .iter()
            .find(|t| t.track_type() == TrackType::Video && t.codec_id() == "V_MPEGH/ISO/HEVC");

        ensure!(track.is_some(), "No HEVC video track found in file");

        let track = track.unwrap();
        let track_id = track.track_number().get();

        let config = if let Some(codec_private) = track.codec_private() {
            let mut bs = BsIoSliceReader::from_slice(codec_private);
            HEVCDecoderConfigurationRecord::parse(&mut bs)?
        } else {
            bail!("Missing HEVC codec private data");
        };

        let nalu_size_length = (config.length_size_minus_one as usize) + 1;
        let mut frame = matroska_demuxer::Frame::default();

        let mut frame_idx = 0;

        let mut frame_nals = Vec::with_capacity(16);

        while let Ok(res) = mkv.next_frame(&mut frame) {
            if !res {
                break;
            } else if frame.track != track_id {
                continue;
            }

            if self.opts.limit.is_some_and(|limit| frame_idx >= limit) {
                // last frame was already processed so we can break
                break;
            }

            frame_idx += 1;

            let data = frame.data.as_slice();
            let mut pos = 0;
            let end = data.len() - 1;

            // Not exactly going to be accurate since only frame data is considered
            self.consumed += data.len();

            while (pos + nalu_size_length) <= end {
                let nalu_size =
                    u32::from_be_bytes(data[pos..pos + nalu_size_length].try_into()?) as usize;

                if nalu_size == 0 {
                    continue;
                } else if (pos + nalu_size) > end {
                    break;
                }

                pos += nalu_size_length;

                let end = pos + nalu_size;
                let parsing_end = if nalu_size > MAX_PARSE_SIZE {
                    pos + MAX_PARSE_SIZE
                } else {
                    end
                };

                let buf = &data[pos..parsing_end];

                let nal = NALUnit {
                    start: pos,
                    end,
                    decoded_frame_index: self.parser.decoded_index,
                    start_code: NALUStartCode::Length4,
                    ..Default::default()
                };

                let nal =
                    self.parser
                        .handle_nal_without_start_code(buf, nal, self.opts.parse_nals)?;
                frame_nals.push(nal);

                pos += nalu_size;
            }

            processor.process_nals(&self.parser, &frame_nals, data)?;
            frame_nals.clear();

            if self.consumed >= 100_000_000 {
                processor.update_progress(1);
                self.consumed = 0;
            }
        }

        self.parser.finish();

        processor.finalize(&self.parser)?;

        Ok(())
    }
}

impl Default for HevcProcessorOpts {
    fn default() -> Self {
        Self {
            buffer_frame: false,
            parse_nals: true,
            limit: Default::default(),
        }
    }
}
