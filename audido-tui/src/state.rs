use audido_core::{commands::AudioResponse, metadata::AudioMetadata};

/// Which widget is currently focused for keyboard navigation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusedWidget {
    Playback,
    Log,
}

/// Application state for the TUI
pub struct AppState {
    /// Currently focused widget for keyboard navigation
    pub focused_widget: FocusedWidget,
    /// Whether audio is currently playing
    pub is_playing: bool,
    /// Current playback position in seconds
    pub position: f32,
    /// Total duration in seconds
    pub duration: f32,
    /// Current volume (0.0 to 1.0)
    pub volume: f32,
    /// Currently loaded audio metadata
    pub metadata: Option<AudioMetadata>,
    /// Status message to display
    pub status_message: String,
    /// Error message if any
    pub error_message: Option<String>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            focused_widget: FocusedWidget::Playback,
            is_playing: false,
            position: 0.0,
            duration: 0.0,
            volume: 1.0,
            metadata: None,
            status_message: "No audio loaded. Pass a file path as argument.".to_string(),
            error_message: None,
        }
    }

    /// Handle response from the audio engine
    pub fn handle_response(&mut self, response: AudioResponse) {
        self.error_message = None;

        match response {
            AudioResponse::Playing => {
                self.is_playing = true;
                self.status_message = "Playing".to_string();
            }
            AudioResponse::Paused => {
                self.is_playing = false;
                self.status_message = "Paused".to_string();
            }
            AudioResponse::Stopped => {
                self.is_playing = false;
                self.position = 0.0;
                self.status_message = "Stopped".to_string();
            }
            AudioResponse::Loaded(metadata) => {
                self.duration = metadata.duration;
                self.status_message = format!(
                    "Loaded: {} - {}",
                    metadata.title.as_deref().unwrap_or("Unknown"),
                    metadata.author.as_deref().unwrap_or("Unknown")
                );
                self.metadata = Some(metadata);
            }
            AudioResponse::Position { current, total } => {
                self.position = current;
                self.duration = total;
            }
            AudioResponse::Error(msg) => {
                self.error_message = Some(msg.clone());
                self.status_message = format!("Error: {}", msg);
            }
            AudioResponse::Shutdown => {
                self.status_message = "Engine shutdown".to_string();
            }
        }
    }

    /// Get the progress percentage (0.0 to 1.0)
    pub fn progress(&self) -> f32 {
        if self.duration > 0.0 {
            (self.position / self.duration).clamp(0.0, 1.0)
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

    /// Toggle focus between Playback and Log widgets
    pub fn toggle_focus(&mut self) {
        self.focused_widget = match self.focused_widget {
            FocusedWidget::Playback => FocusedWidget::Log,
            FocusedWidget::Log => FocusedWidget::Playback,
        };
    }
}
