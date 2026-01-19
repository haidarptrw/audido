use std::{fs::File, path::Path, time::Instant};

use anyhow::Context;
use rodio::{Decoder, Source};

use crate::metadata::{AudioMetadata, ChannelLayout};

pub struct AudioPlaybackData {
    metadata: AudioMetadata,
    buffer: Vec<f32>,
    buffer_size: usize,
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

        // analyze
        Self::get_audio_properties(&samples, &mut metadata)?;

        let playback_data = AudioPlaybackData {
            metadata,
            buffer: samples,
            buffer_size: 512,
        };

        log::debug!("Load audio finished in {:?} seconds", start_time.elapsed());
        Ok(playback_data)
    }

    /// Get audio properties from a buffer and then assign it to the metadata
    fn get_audio_properties(buffer: &[f32], metadata: &mut AudioMetadata) -> anyhow::Result<()> {
        Ok(())
    }
}
