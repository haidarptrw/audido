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
use lofty::{file::TaggedFileExt, probe::Probe, tag::Accessor};
use rodio::{Decoder, Source};

use crate::metadata::{AudioMetadata, ChannelLayout};

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
        Self::get_audio_properties(&samples, &mut metadata)?;

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
    fn get_audio_properties(buffer: &[f32], metadata: &mut AudioMetadata) -> anyhow::Result<()> {
        Ok(())
    }

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
    pub fn create_source(&self) -> BufferedSource {
        BufferedSource {
            samples: Arc::clone(&self.buffer),
            sample_rate: self.metadata.sample_rate,
            channels: self.metadata.num_channels,
            position_tracker: self.position_tracker.clone(),
        }
    }
}

/// A buffered audio source that implements rodio's Source trait
pub struct BufferedSource {
    samples: Arc<Vec<f32>>,
    sample_rate: u32,
    channels: u16,
    position_tracker: PositionTracker,
}

impl Iterator for BufferedSource {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        let pos = self.position_tracker.position.load(Ordering::Relaxed);
        if pos < self.samples.len() {
            let sample = self.samples[pos];
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

mod test {
    pub fn test_loading_audio() {}

    pub fn test_reading_metadata() {}

    pub fn test_audio_analysis() {}
}
