use std::{
    fs::File,
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Instant,
};

use anyhow::Context;
use crossbeam_channel::Receiver;
use lofty::{file::TaggedFileExt, probe::Probe, tag::Accessor};
use rodio::{Decoder, Source};

use crate::{
    commands::RealtimeAudioCommand,
    dsp::{dsp_graph::DspNode, eq::Equalizer},
    metadata::{AudioMetadata, ChannelLayout},
};

const CHUNK_SIZE: usize = 512;

/// Shared position tracker between source and engine
#[derive(Clone)]
pub struct PositionTracker {
    /// Current sample position (atomic for thread-safe access)
    position: Arc<AtomicUsize>,
    /// Total number of samples
    total_samples: usize,
    /// Sample rate for time calculations
    sample_rate: u32,
    /// Number of channels
    channels: u16,
}

impl PositionTracker {
    pub fn new(total_samples: usize, sample_rate: u32, channels: u16) -> Self {
        Self {
            position: Arc::new(AtomicUsize::new(0)),
            total_samples,
            sample_rate,
            channels,
        }
    }

    /// Get current position in seconds
    pub fn position_seconds(&self) -> f32 {
        let pos = self.position.load(Ordering::Relaxed);
        let frames = pos / self.channels as usize;
        frames as f32 / self.sample_rate as f32
    }

    /// Get total duration in seconds
    pub fn duration_seconds(&self) -> f32 {
        let frames = self.total_samples / self.channels as usize;
        frames as f32 / self.sample_rate as f32
    }

    /// Set position from seconds
    pub fn seek_to_seconds(&self, seconds: f32) {
        let frames = (seconds * self.sample_rate as f32) as usize;
        let sample_pos = (frames * self.channels as usize).min(self.total_samples);
        self.position.store(sample_pos, Ordering::Relaxed);
    }

    /// Reset position to start
    pub fn reset(&self) {
        self.position.store(0, Ordering::Relaxed);
    }
}

pub struct AudioPlaybackData {
    metadata: AudioMetadata,
    buffer: Arc<Vec<f32>>,
    position_tracker: PositionTracker,
}

pub enum AudioSource {
    Local { data: AudioPlaybackData },
}

impl AudioPlaybackData {
    pub fn load_local_audio(path: &str) -> anyhow::Result<AudioPlaybackData> {
        // calculate time required for performance monitoring
        let start_time = Instant::now();

        let file = File::open(path).context("Failed to open the file")?;
        let decoder = Decoder::try_from(file).context("Failed to decode the opened audio file")?;

        let sample_rate = decoder.sample_rate();
        let num_channels = decoder.channels();

        let channel_layout = match num_channels {
            1 => ChannelLayout::Mono,
            2 => ChannelLayout::Stereo,
            _ => ChannelLayout::Unsupported,
        };

        log::debug!("Starting full decode with rodio.");
        let samples: Vec<f32> = decoder.collect();
        log::debug!("Finished decoding {} samples.", samples.len());

        let n_frames = (samples.len() / num_channels as usize) as u32;
        let duration_in_seconds = if sample_rate > 0 {
            n_frames as f32 / sample_rate as f32
        } else {
            0.0
        };

        let file_ext = Path::new(path)
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string();

        let mut metadata = AudioMetadata {
            sample_rate,
            num_channels,
            channel_layout,
            duration: duration_in_seconds,
            format: file_ext,
            ..Default::default()
        };

        // read metadata
        Self::get_audio_metadata(path, &mut metadata)?;

        // analyze
        Self::get_audio_properties(&samples, num_channels, &mut metadata)?;

        let total_samples = samples.len();
        let position_tracker = PositionTracker::new(total_samples, sample_rate, num_channels);

        let playback_data = AudioPlaybackData {
            metadata,
            buffer: Arc::new(samples),
            position_tracker,
        };

        log::debug!("Load audio finished in {:?} seconds", start_time.elapsed());
        Ok(playback_data)
    }

    /// Get audio properties from a buffer and then assign it to the metadata
    #[allow(unused_variables)]
    fn get_audio_properties(
        buffer: &[f32],
        num_channels: u16,
        metadata: &mut AudioMetadata,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    //// Get audio metadata from loaded file (title, author, album, genre, etc)
    fn get_audio_metadata(path: &str, metadata: &mut AudioMetadata) -> anyhow::Result<()> {
        match Probe::open(path).and_then(|p| p.read()) {
            Ok(tagged_file) => {
                if let Some(tag) = tagged_file.primary_tag() {
                    metadata.title = tag.title().map(|s| s.to_string());
                    metadata.author = tag.artist().map(|s| s.to_string());
                    metadata.album = tag.album().map(|s| s.to_string());
                    metadata.genre = tag.genre().map(|s| s.to_string());

                    log::info!(
                        "Metadata loaded: {:?} by {:?}",
                        metadata.title,
                        metadata.author
                    );
                }
            }
            Err(e) => {
                log::warn!("Failed to read metadata: {}", e);
            }
        }

        if metadata.title.is_none() {
            metadata.title = Path::new(path)
                .file_stem()
                .and_then(|s| s.to_str())
                .map(|s| s.to_string());
        }
        Ok(())
    }

    /// Get a reference to the audio metadata
    pub fn metadata(&self) -> &AudioMetadata {
        &self.metadata
    }

    /// Get a reference to the position tracker
    pub fn position_tracker(&self) -> &PositionTracker {
        &self.position_tracker
    }

    /// Create a rodio Source from the buffered audio data
    pub fn create_source(
        &self,
        initial_eq: Equalizer,
        cmd_rx: Receiver<RealtimeAudioCommand>,
    ) -> BufferedSource {
        BufferedSource::new(
            self.buffer.clone(),
            self.metadata.sample_rate,
            self.metadata.num_channels,
            self.position_tracker.clone(),
            initial_eq,
            cmd_rx,
        )
    }
}

/// A buffered audio source that implements rodio's Source trait
pub struct BufferedSource {
    samples: Arc<Vec<f32>>,
    sample_rate: u32,
    channels: u16,
    position_tracker: PositionTracker,
    equalizer: DspNode<Equalizer>,
    cmd_rx: Receiver<RealtimeAudioCommand>,

    // Chunk Processing
    process_buffer: Vec<f32>,
    process_buffer_idx: usize,
}

impl BufferedSource {
    pub fn new(
        samples: Arc<Vec<f32>>,
        sample_rate: u32,
        channels: u16,
        position_tracker: PositionTracker,
        equalizer: Equalizer,
        cmd_rx: Receiver<RealtimeAudioCommand>,
    ) -> Self {
        Self {
            samples,
            sample_rate,
            channels,
            position_tracker,
            equalizer: DspNode::new(equalizer),
            cmd_rx,
            process_buffer: Vec::with_capacity(CHUNK_SIZE),
            process_buffer_idx: 0,
        }
    }

    fn fill_buffer(&mut self) -> bool {
        self.process_buffer.clear();
        self.process_buffer_idx = 0;

        // 1. Process Pending EQ Commands (Lock-Free)
        while let Ok(cmd) = self.cmd_rx.try_recv() {
            match cmd {
                RealtimeAudioCommand::UpdateEqFilter(idx, filter_node) => {
                    self.equalizer.set_filter(idx, filter_node);
                }
                RealtimeAudioCommand::SetAllEqFilters(filter_nodes) => {
                    self.equalizer.set_all_filters(filter_nodes);
                }
                RealtimeAudioCommand::SetEqMasterGain(gain) => {
                    self.equalizer.set_master_gain(gain);
                }
                RealtimeAudioCommand::SetEqPreset(preset) => {
                    self.equalizer.instance.update_preset(preset);
                }
                RealtimeAudioCommand::SetEqEnabled(enabled) => {
                    self.equalizer.on = enabled;
                }
            }
        }

        // 2. Fetch Audio
        let global_pos = self.position_tracker.position.load(Ordering::Relaxed);
        if global_pos >= self.samples.len() {
            return false;
        }

        let end_pos = (global_pos + CHUNK_SIZE).min(self.samples.len());
        self.process_buffer
            .extend_from_slice(&self.samples[global_pos..end_pos]);

        // 3. Apply DSP only if EQ is enabled
        if self.equalizer.on {
            self.equalizer
                .instance
                .process_frame(&mut self.process_buffer);
        }

        true
    }
}

impl Iterator for BufferedSource {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        // If we've exhausted the process buffer, refill it
        if self.process_buffer_idx >= self.process_buffer.len() {
            if !self.fill_buffer() {
                return None;
            }
        }

        // Return the next sample from our processed buffer
        if self.process_buffer_idx < self.process_buffer.len() {
            let sample = self.process_buffer[self.process_buffer_idx];
            self.process_buffer_idx += 1;

            // Update position tracker
            let pos = self.position_tracker.position.load(Ordering::Relaxed);
            self.position_tracker
                .position
                .store(pos + 1, Ordering::Relaxed);

            Some(sample)
        } else {
            None
        }
    }
}

impl Source for BufferedSource {
    fn current_span_len(&self) -> Option<usize> {
        let pos = self.position_tracker.position.load(Ordering::Relaxed);
        Some(self.samples.len() - pos)
    }

    fn channels(&self) -> u16 {
        self.channels
    }

    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    fn total_duration(&self) -> Option<std::time::Duration> {
        let frames = self.samples.len() / self.channels as usize;
        Some(std::time::Duration::from_secs_f64(
            frames as f64 / self.sample_rate as f64,
        ))
    }
}

// mod test {
//     pub fn test_loading_audio() {}

//     pub fn test_reading_metadata() {}

//     pub fn test_audio_analysis() {}
// }
