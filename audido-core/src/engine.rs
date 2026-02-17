use std::thread;
use std::time::Duration;

use anyhow::Context;
use crossbeam_channel::{Receiver, Sender, unbounded};
use rodio::{
    DeviceTrait, OutputStream, OutputStreamBuilder, Sink,
    cpal::{self, traits::HostTrait},
};

use crate::queue::{LoopMode, PlaybackQueue};
use crate::source::AudioPlaybackData;
use crate::{
    commands::{AudioCommand, AudioResponse, RealtimeAudioCommand},
    dsp::eq::Equalizer,
};

/// Handle to communicate with the audio engine from the TUI
pub struct AudioEngineHandle {
    pub cmd_tx: Sender<AudioCommand>,
    pub resp_rx: Receiver<AudioResponse>,
}

pub struct AudioEngine {
    _stream: OutputStream,
    sink: Sink,
    device_name: String,
    cmd_rx: Receiver<AudioCommand>,
    resp_tx: Sender<AudioResponse>,
    current_audio: Option<AudioPlaybackData>,
    is_playing: bool,
    target_volume: f32,
    queue: PlaybackQueue,
    // Realtime audio command sender (receiver is owned by BufferedSource)
    eq_shadow: Equalizer,
    eq_enabled: bool,
    rt_cmd_tx: Option<Sender<RealtimeAudioCommand>>,
}

// Constants for fading
const FADE_DURATION_MS: u64 = 100;
const FADE_STEPS: u32 = 20;
const FADE_STEP_DURATION: Duration = Duration::from_millis(FADE_DURATION_MS / FADE_STEPS as u64);

impl AudioEngine {
    /// Create a new audio engine and return a handle for communication
    pub fn new() -> anyhow::Result<(Self, AudioEngineHandle)> {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .context("No default output device found")?;

        let device_name = device.name().unwrap_or_else(|_| "(unknown)".to_string());

        let stream_builder = OutputStreamBuilder::from_device(device)
            .context("Cannot create output stream builder from device")?;

        let stream = stream_builder
            .open_stream()
            .context("Cannot create stream output")?;

        let sink = Sink::connect_new(stream.mixer());

        // Create crossbeam channels
        let (cmd_tx, cmd_rx) = unbounded::<AudioCommand>();
        let (resp_tx, resp_rx) = unbounded::<AudioResponse>();

        let engine = AudioEngine {
            _stream: stream,
            sink,
            device_name,
            cmd_rx,
            resp_tx,
            current_audio: None,
            is_playing: false,
            target_volume: 1.0,
            queue: PlaybackQueue::new(),
            eq_shadow: Equalizer::new(44100, 2),
            eq_enabled: false,
            rt_cmd_tx: None,
        };

        let handle = AudioEngineHandle { cmd_tx, resp_rx };

        log::info!(
            "Audio engine initialized with device: {}",
            engine.device_name
        );

        Ok((engine, handle))
    }

    /// Spawn the audio engine on a dedicated thread
    pub fn spawn(self) -> thread::JoinHandle<()> {
        thread::spawn(move || {
            self.run();
        })
    }

    /// Helper to fade volume from current level down to 0
    fn perform_fade_out(&self) {
        if self.sink.empty() || self.sink.is_paused() {
            return;
        }

        // We fade out from the current user target volume (or current sink volume)
        // just to be safe, let's start from whatever the sink currently has.
        let start_vol = self.sink.volume();

        if start_vol <= 0.001 {
            return;
        }

        for i in 1..=FADE_STEPS {
            let progress = i as f32 / FADE_STEPS as f32;
            let vol = start_vol * (1.0 - progress);
            self.sink.set_volume(vol);
            thread::sleep(FADE_STEP_DURATION);
        }
        self.sink.set_volume(0.0);
    }

    /// Helper to fade volume from 0 up to target_volume
    fn perform_fade_in(&self) {
        // Ensure we start at 0
        self.sink.set_volume(0.0);

        let target = self.target_volume;
        if target <= 0.001 {
            return;
        }

        for i in 1..=FADE_STEPS {
            let progress = i as f32 / FADE_STEPS as f32;
            let vol = target * progress;
            self.sink.set_volume(vol);
            thread::sleep(FADE_STEP_DURATION);
        }
        // Ensure we hit the exact target at the end
        self.sink.set_volume(target);
    }

    /// Main engine loop - processes commands and updates playback state
    pub fn run(mut self) {
        log::info!("Audio engine started");

        loop {
            // Check for commands (non-blocking with timeout)
            match self.cmd_rx.recv_timeout(Duration::from_millis(50)) {
                Ok(cmd) => {
                    if !self.process_command(cmd) {
                        break; // Quit command received
                    }
                }
                Err(crossbeam_channel::RecvTimeoutError::Timeout) => {
                    // No command, continue
                }
                Err(crossbeam_channel::RecvTimeoutError::Disconnected) => {
                    log::info!("Command channel disconnected, shutting down");
                    break;
                }
            }

            if self.is_playing && self.sink.empty() && !self.sink.is_paused() {
                log::info!("Track finished naturally.");

                if let Some(ref audio_data) = self.current_audio {
                    audio_data.position_tracker().reset();
                }

                // Try to advance to next track based on loop mode
                if let Some(next_idx) = self.queue.next_index() {
                    self.play_queue_track(next_idx);
                } else {
                    // No more tracks, stop playback
                    self.is_playing = false;
                    let _ = self.resp_tx.send(AudioResponse::Stopped);
                    let _ = self.resp_tx.send(AudioResponse::Position {
                        current: 0.0,
                        total: 0.0,
                    });
                    self.sink.set_volume(self.target_volume);
                }
            }

            // Send position updates if playing
            if !self.sink.is_paused()
                && !self.sink.empty()
                && let Some(ref audio_data) = self.current_audio
            {
                let tracker = audio_data.position_tracker();
                let current = tracker.position_seconds();
                let total = tracker.duration_seconds();
                let _ = self
                    .resp_tx
                    .send(AudioResponse::Position { current, total });
            }
        }

        log::info!("Audio engine stopped");
        let _ = self.resp_tx.send(AudioResponse::Shutdown);
    }

    /// Process a single command, returns false if engine should quit
    fn process_command(&mut self, cmd: AudioCommand) -> bool {
        match cmd {
            AudioCommand::Load(path) => {
                log::info!("Loading audio: {}", path);

                if self.is_playing {
                    self.perform_fade_out();
                }

                self.sink.stop();
                self.is_playing = false;

                match AudioPlaybackData::load_local_audio(&path) {
                    Ok(audio_data) => {
                        let metadata = audio_data.metadata().clone();

                        let previous_filters = self.eq_shadow.filters.clone();
                        let previous_gain = self.eq_shadow.master_gain;
                        let previous_preset = self.eq_shadow.preset;

                        let mut new_eq =
                            Equalizer::new(metadata.sample_rate, metadata.num_channels);

                        new_eq.filters = previous_filters;
                        new_eq.master_gain = previous_gain;
                        new_eq.preset = previous_preset;

                        new_eq.parameters_changed();
                        self.eq_shadow = new_eq;

                        self.current_audio = Some(audio_data);
                        let _ = self.resp_tx.send(AudioResponse::Loaded(metadata.clone()));

                        if let Some(ref data) = self.current_audio {
                            // Create realtime audio command channel
                            let (rt_tx, rt_rx) = unbounded::<RealtimeAudioCommand>();
                            self.rt_cmd_tx = Some(rt_tx);

                            self.sink.append(data.create_source(
                                self.eq_shadow.clone(),
                                self.eq_enabled,
                                rt_rx,
                            ));
                            self.sink.set_volume(0.0);
                            self.sink.play();
                            self.is_playing = true;
                            let _ = self.resp_tx.send(AudioResponse::Playing);
                            self.perform_fade_in();
                        }
                    }
                    Err(e) => {
                        let _ = self
                            .resp_tx
                            .send(AudioResponse::Error(format!("Failed to load audio: {}", e)));
                    }
                }
            }
            AudioCommand::Play => {
                if let Some(ref audio_data) = self.current_audio {
                    // Append audio source to sink if not already playing
                    if self.sink.empty() {
                        let (rt_tx, rt_rx) = unbounded::<RealtimeAudioCommand>();
                        self.rt_cmd_tx = Some(rt_tx);
                        self.sink.append(audio_data.create_source(
                            self.eq_shadow.clone(),
                            self.eq_enabled,
                            rt_rx,
                        ));
                    }
                    if !self.is_playing {
                        self.sink.set_volume(0.0);
                        self.sink.play();
                        self.is_playing = true;
                        let _ = self.resp_tx.send(AudioResponse::Playing);
                        self.perform_fade_in();
                    }
                } else {
                    let _ = self
                        .resp_tx
                        .send(AudioResponse::Error("No audio loaded".to_string()));
                }
            }
            AudioCommand::Pause => {
                if self.is_playing {
                    // Fade out
                    self.perform_fade_out();

                    self.sink.pause();
                    self.is_playing = false;
                    let _ = self.resp_tx.send(AudioResponse::Paused);
                }
            }
            AudioCommand::Stop => {
                if self.is_playing {
                    self.perform_fade_out();
                }

                self.sink.stop();
                self.is_playing = false;
                // Reset position tracker
                if let Some(ref audio_data) = self.current_audio {
                    audio_data.position_tracker().reset();
                }
                self.sink.set_volume(self.target_volume);
                let _ = self.resp_tx.send(AudioResponse::Stopped);
            }
            AudioCommand::SetVolume(volume) => {
                let clamped = volume.clamp(0.0, 1.0);
                self.target_volume = clamped;
                if self.is_playing {
                    self.sink.set_volume(clamped);
                }
            }
            AudioCommand::SetSpeed(speed) => {
                self.sink.set_speed(speed.clamp(0.1, 4.0));
            }
            AudioCommand::Seek(pos) => {
                if let Some(ref audio_data) = self.current_audio {
                    // Check previous state logic (updated to use is_playing flag)
                    let should_play = self.is_playing;

                    // Stop current playback stream to clear buffer
                    self.sink.stop();

                    // Update position tracker
                    audio_data.position_tracker().seek_to_seconds(pos);

                    // Create and append new source (starts from tracked position)
                    let (rt_tx, rt_rx) = unbounded::<RealtimeAudioCommand>();
                    self.rt_cmd_tx = Some(rt_tx);
                    self.sink.append(audio_data.create_source(
                        self.eq_shadow.clone(),
                        self.eq_enabled,
                        rt_rx,
                    ));

                    if should_play {
                        self.sink.set_volume(self.target_volume);
                        self.sink.play();
                    } else {
                        self.sink.pause();
                    }

                    log::info!("Seeked to {} seconds", pos);
                }
            }
            AudioCommand::Next => {
                if let Some(next_idx) = self.queue.next_index() {
                    log::info!("Skipping to next track (index {})", next_idx);
                    self.play_queue_track(next_idx);
                } else {
                    log::info!("No next track available");
                }
            }
            AudioCommand::Previous => {
                if let Some(prev_idx) = self.queue.prev_index() {
                    log::info!("Skipping to previous track (index {})", prev_idx);
                    self.play_queue_track(prev_idx);
                } else {
                    log::info!("No previous track available");
                }
            }
            AudioCommand::Quit => {
                if self.is_playing {
                    self.perform_fade_out();
                }
                log::info!("Quit command received");
                self.sink.stop();
                return false;
            }
            AudioCommand::AddToQueue(paths) => {
                log::info!("Adding {} items to queue", paths.len());
                let was_empty = self.queue.items.is_empty();
                let path_bufs: Vec<std::path::PathBuf> =
                    paths.into_iter().map(|s| s.into()).collect();
                self.queue.add(path_bufs);

                // Auto-play if not already playing and not paused
                if !self.is_playing && !self.sink.is_paused() {
                    if was_empty {
                        self.play_queue_track(0);
                    } else if let Some(next_idx) = self.queue.next_index() {
                        // This handles the case where the queue had ended
                        self.play_queue_track(next_idx);
                    }
                }

                self.send_queue_update();
            }
            AudioCommand::RemoveFromQueue(id) => {
                if self.queue.remove(id) {
                    log::info!("Removed item {} from queue", id);
                    self.send_queue_update();
                }
            }
            AudioCommand::ClearQueue => {
                log::info!("Clearing queue");
                if self.is_playing {
                    self.perform_fade_out();
                    self.sink.stop();
                    self.is_playing = false;
                }
                self.queue.clear();
                self.current_audio = None;
                self.send_queue_update();
                let _ = self.resp_tx.send(AudioResponse::Stopped);
            }
            AudioCommand::SetLoopMode(mode) => {
                log::info!("Setting loop mode to {:?}", mode);
                self.queue.loop_mode = mode;
                if mode == LoopMode::Shuffle {
                    self.queue.reshuffle();
                }
                let _ = self.resp_tx.send(AudioResponse::LoopModeChanged(mode));
            }
            AudioCommand::PlayQueueIndex(index) => {
                if index < self.queue.items.len() {
                    log::info!("Playing queue index {}", index);
                    self.play_queue_track(index);
                } else {
                    let _ = self.resp_tx.send(AudioResponse::Error(format!(
                        "Invalid queue index: {}",
                        index
                    )));
                }
            }
            AudioCommand::EqSetEnabled(enabled) => {
                log::info!("Setting EQ enabled: {}", enabled);
                self.eq_enabled = enabled;
                if let Some(ref tx) = self.rt_cmd_tx {
                    let _ = tx.send(RealtimeAudioCommand::SetEqEnabled(enabled));
                }
            }
            AudioCommand::EqSetMasterGain(gain_db) => {
                log::info!("Setting EQ master gain: {} dB", gain_db);
                // Convert dB to linear gain
                let linear_gain = 10.0f32.powf(gain_db / 20.0);
                self.eq_shadow.master_gain = linear_gain;
                if let Some(ref tx) = self.rt_cmd_tx {
                    let _ = tx.send(RealtimeAudioCommand::SetEqMasterGain(linear_gain));
                }
            }
            AudioCommand::EqSetPreset(eq_preset) => {
                log::info!("Setting EQ preset: {:?}", eq_preset);
                self.eq_shadow.preset = eq_preset;
                self.eq_shadow.parameters_changed();
                if let Some(ref tx) = self.rt_cmd_tx {
                    let _ = tx.send(RealtimeAudioCommand::SetEqPreset(eq_preset));
                }
            }
            AudioCommand::EqSetAllFilters(filters) => {
                log::info!("Setting all EQ filters: {} bands", filters.len());
                self.eq_shadow.filters = filters.clone();
                self.eq_shadow.parameters_changed();
                if let Some(ref tx) = self.rt_cmd_tx {
                    let _ = tx.send(RealtimeAudioCommand::SetAllEqFilters(filters));
                }
            }
            AudioCommand::EqResetParameters => {
                log::info!("Setting all EQ filters to their default state");
                self.eq_shadow.reset_parameters();
                self.eq_shadow.parameters_changed();
                if let Some(ref tx) = self.rt_cmd_tx {
                    let _ = tx.send(RealtimeAudioCommand::ResetEq);
                }
            }
            AudioCommand::EqResetFilterNode(index) => {
                log::info!("Resetting EQ filter node {} to preset default", index);
                if let Err(e) = self.eq_shadow.reset_filter_node_param(index) {
                    log::warn!("Failed to reset filter node {}: {}", index, e);
                }
                if let Some(ref tx) = self.rt_cmd_tx {
                    let _ = tx.send(RealtimeAudioCommand::ResetEqFilterNode(index));
                }
            }
        }
        true
    }

    /// Helper to play a track from the queue by index
    fn play_queue_track(&mut self, index: usize) {
        if let Some(item) = self.queue.get(index) {
            let path = item.path.to_string_lossy().to_string();

            // Fade out current track if playing
            if self.is_playing {
                self.perform_fade_out();
            }
            self.sink.stop();
            self.is_playing = false;

            // Load the new track
            match AudioPlaybackData::load_local_audio(&path) {
                Ok(audio_data) => {
                    let metadata = audio_data.metadata().clone();

                    // Update queue metadata
                    self.queue.set_metadata(item.id, metadata.clone());
                    self.queue.current_index = Some(index);

                    self.current_audio = Some(audio_data);

                    // Send track changed notification
                    let _ = self.resp_tx.send(AudioResponse::TrackChanged {
                        index,
                        metadata: metadata.clone(),
                    });

                    // Preserve existing EQ settings for the new track
                    let previous_filters = self.eq_shadow.filters.clone();
                    let previous_gain = self.eq_shadow.master_gain;
                    let previous_preset = self.eq_shadow.preset;

                    let mut new_eq = Equalizer::new(metadata.sample_rate, metadata.num_channels);

                    new_eq.filters = previous_filters;
                    new_eq.master_gain = previous_gain;
                    new_eq.preset = previous_preset;

                    new_eq.parameters_changed();
                    self.eq_shadow = new_eq;

                    let _ = self.resp_tx.send(AudioResponse::Loaded(metadata));
                    // Start playing
                    if let Some(ref data) = self.current_audio {
                        let (rt_tx, rt_rx) = unbounded::<RealtimeAudioCommand>();
                        self.rt_cmd_tx = Some(rt_tx);
                        self.sink.append(data.create_source(
                            self.eq_shadow.clone(),
                            self.eq_enabled,
                            rt_rx,
                        ));
                        self.sink.set_volume(0.0);
                        self.sink.play();
                        self.is_playing = true;
                        let _ = self.resp_tx.send(AudioResponse::Playing);
                        self.perform_fade_in();
                    }
                }
                Err(e) => {
                    let _ = self
                        .resp_tx
                        .send(AudioResponse::Error(format!("Failed to load track: {}", e)));
                }
            }
        }
    }

    /// Send queue update to TUI
    fn send_queue_update(&self) {
        let _ = self
            .resp_tx
            .send(AudioResponse::QueueUpdated(self.queue.items.clone()));
    }
}
