use anyhow::Result;
use std::io::Read;

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

    /// Parse NALUs from the stream
    /// Depending on the options, this either:
    ///   - Loops the entire stream until EOF
    ///   - Loops until a complete frame has been parsed
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

            if self.opts.buffer_frame {
                let next_frame = nals.iter().map(|nal| nal.decoded_frame_index).max();

                if let Some(number) = next_frame {
                    if number > self.last_buffered_frame {
                        self.last_buffered_frame = number;

                        // Stop reading
                        break;
                    }
                }
            }
        }

        Ok(())
    }
}

impl Default for HevcProcessorOpts {
    fn default() -> Self {
        Self {
            buffer_frame: false,
            parse_nals: true,
        }
    }
}
