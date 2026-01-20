use crate::metadata::AudioMetadata;

/// Commands sent from the TUI to the audio engine
#[derive(Debug, Clone)]
pub enum AudioCommand {
    /// Load an audio file from the given path
    Load(String),
    /// Start or resume playback
    Play,
    /// Pause playback
    Pause,
    /// Stop playback and reset position
    Stop,
    /// Skip to next track (if playlist exists)
    Next,
    /// Skip to previous track (if playlist exists)
    Previous,
    /// Seek to position in seconds
    Seek(f32),
    /// Set volume (0.0 to 1.0)
    SetVolume(f32),
    /// Set playback speed multiplier
    SetSpeed(f32),
    /// Shutdown the audio engine
    Quit,
}

/// Responses sent from the audio engine to the TUI
#[derive(Debug, Clone)]
pub enum AudioResponse {
    /// Playback has started
    Playing,
    /// Playback has been paused
    Paused,
    /// Playback has been stopped
    Stopped,
    /// Audio file loaded successfully with metadata
    Loaded(AudioMetadata),
    /// Current playback position in seconds and total duration
    Position { current: f32, total: f32 },
    /// An error occurred
    Error(String),
    /// Engine is shutting down
    Shutdown,
}
