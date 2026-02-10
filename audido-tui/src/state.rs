use audido_core::{commands::AudioResponse, queue::LoopMode};

use crate::states::{
    AudioState, BrowserState, EqState, QueueState,
    SettingsState,
};

/// Application state for the TUI
pub struct AppState {
    /// Audio playback state
    pub audio: AudioState,
    /// Browser state
    pub browser_state: BrowserState,
    /// Queue state
    pub queue: QueueState,
    /// EQ State
    pub eq_state: EqState,
    /// Settings State
    pub settings_state: SettingsState,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            audio: AudioState::new(),
            browser_state: BrowserState::new(),
            queue: QueueState::new(),
            eq_state: EqState::new(),
            settings_state: SettingsState::new(),
        }
    }

    /// Handle response from the audio engine
    pub fn handle_response(&mut self, response: AudioResponse) {
        self.audio.error_message = None;

        match response {
            AudioResponse::Playing => {
                self.audio.is_playing = true;
                self.audio.status_message = "Playing".to_string();
            }
            AudioResponse::Paused => {
                self.audio.is_playing = false;
                self.audio.status_message = "Paused".to_string();
            }
            AudioResponse::Stopped => {
                self.audio.is_playing = false;
                self.audio.position = 0.0;
                self.audio.status_message = "Stopped".to_string();
            }
            AudioResponse::Loaded(metadata) => {
                self.audio.duration = metadata.duration;
                self.audio.status_message = format!(
                    "Loaded: {} - {}",
                    metadata.title.as_deref().unwrap_or("Unknown"),
                    metadata.author.as_deref().unwrap_or("Unknown")
                );
                self.audio.metadata = Some(metadata);
            }
            AudioResponse::Position { current, total } => {
                self.audio.position = current;
                self.audio.duration = total;
            }
            AudioResponse::Error(msg) => {
                self.audio.error_message = Some(msg.clone());
                self.audio.status_message = format!("Error: {}", msg);
            }
            AudioResponse::Shutdown => {
                self.audio.status_message = "Engine shutdown".to_string();
            }
            AudioResponse::QueueUpdated(queue_items) => {
                self.queue.queue = queue_items;
                if !self.queue.queue.is_empty() && self.queue.queue_state.selected().is_none() {
                    self.queue.queue_state.select(Some(0));
                }
            }
            AudioResponse::LoopModeChanged(mode) => {
                self.queue.loop_mode = mode;
            }
            AudioResponse::TrackChanged { index, metadata } => {
                self.queue.current_queue_index = Some(index);
                self.queue.queue_state.select(Some(index));
                self.audio.metadata = Some(metadata);
                self.audio.status_message =
                    format!("Track {}/{}", index + 1, self.queue.queue.len());
            }
        }
    }

    /// Get the progress percentage (0.0 to 1.0)
    pub fn progress(&self) -> f32 {
        if self.audio.duration > 0.0 {
            (self.audio.position / self.audio.duration).clamp(0.0, 1.0)
        } else {
            0.0
        }
    }

    /// Format time as MM:SS
    pub fn format_time(seconds: f32) -> String {
        let mins = (seconds / 60.0).floor() as u32;
        let secs = (seconds % 60.0).floor() as u32;
        format!("{:02}:{:02}", mins, secs)
    }

    // ==============================================
    // Queue Navigation Methods
    // ==============================================

    pub fn queue_next(&mut self) {
        if self.queue.queue.is_empty() {
            return;
        }
        let i = match self.queue.queue_state.selected() {
            Some(i) => {
                if i >= self.queue.queue.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.queue.queue_state.select(Some(i));
    }

    pub fn queue_prev(&mut self) {
        if self.queue.is_empty() {
            return;
        }
        let i = match self.queue.queue_state.selected() {
            Some(i) => {
                if i == 0 {
                    self.queue.queue.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.queue.queue_state.select(Some(i));
    }

    /// Get currently selected queue index
    pub fn queue_selected(&self) -> Option<usize> {
        self.queue.queue_state.selected()
    }

    // ==============================================
    // Loop Mode Methods
    // ==============================================

    /// Cycle to the next loop mode
    pub fn next_loop_mode(&self) -> LoopMode {
        match self.queue.loop_mode {
            LoopMode::Off => LoopMode::RepeatOne,
            LoopMode::RepeatOne => LoopMode::LoopAll,
            LoopMode::LoopAll => LoopMode::Shuffle,
            LoopMode::Shuffle => LoopMode::Off,
        }
    }
}
