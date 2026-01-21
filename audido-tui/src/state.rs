use std::path::PathBuf;

use audido_core::{
    browser::{self, FileEntry},
    commands::AudioResponse,
    metadata::AudioMetadata,
    queue::{LoopMode, QueueItem},
};
use ratatui::widgets::ListState;
use strum::IntoEnumIterator;
use strum_macros::EnumIter;

/// Which tab is currently active in the sidebar navigation
#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumIter, strum_macros::Display)]
pub enum ActiveTab {
    Playback,
    Queue,
    Browser,
    Log,
}

/// Dialog shown when selecting a file in browser
#[derive(Debug, Clone, Default)]
pub enum BrowserFileDialog {
    #[default]
    None,
    /// Dialog open with path and selected option (0=Play Now, 1=Add to Queue)
    Open { path: PathBuf, selected: usize },
}

/// Application state for the TUI
pub struct AppState {
    /// Currently active tab in the sidebar navigation
    pub active_tab: ActiveTab,

    // ==============================
    // Audio State
    // ==============================
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

    // ==============================
    // Browser State
    // ==============================
    pub current_dir: PathBuf,
    pub browser_items: Vec<FileEntry>,
    pub browser_state: ListState,

    // ==============================
    // Queue State
    // ==============================
    pub queue: Vec<QueueItem>,
    pub current_queue_index: Option<usize>,
    pub loop_mode: LoopMode,
    pub queue_state: ListState,

    // ==============================
    // Dialog State
    // ==============================
    pub browser_dialog: BrowserFileDialog,
}

impl AppState {
    pub fn new() -> Self {
        // Initialize browser at current working directory
        let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let browser_items = browser::get_directory_content(&current_dir).unwrap_or_default();
        let mut browser_state = ListState::default();
        if !browser_items.is_empty() {
            browser_state.select(Some(0));
        }

        Self {
            active_tab: ActiveTab::Playback,
            is_playing: false,
            position: 0.0,
            duration: 0.0,
            volume: 1.0,
            metadata: None,
            status_message: "No audio loaded. Pass a file path as argument.".to_string(),
            error_message: None,
            current_dir,
            browser_items,
            browser_state,

            // Queue State
            queue: Vec::new(),
            current_queue_index: None,
            loop_mode: LoopMode::Off,
            queue_state: ListState::default(),

            // Dialog State
            browser_dialog: BrowserFileDialog::None,
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
            AudioResponse::QueueUpdated(queue_items) => {
                self.queue = queue_items;
                if !self.queue.is_empty() && self.queue_state.selected().is_none() {
                    self.queue_state.select(Some(0));
                }
            }
            AudioResponse::LoopModeChanged(mode) => {
                self.loop_mode = mode;
            }
            AudioResponse::TrackChanged { index, metadata } => {
                self.current_queue_index = Some(index);
                self.queue_state.select(Some(index));
                self.metadata = Some(metadata);
                self.status_message = format!("Track {}/{}", index + 1, self.queue.len());
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

    /// Cycle to the next tab in the sidebar navigation
    pub fn next_tab(&mut self) {
        let tabs: Vec<ActiveTab> = ActiveTab::iter().collect();
        let current_idx = tabs.iter().position(|t| *t == self.active_tab).unwrap_or(0);
        let next_idx = (current_idx + 1) % tabs.len();
        self.active_tab = tabs[next_idx];
    }

    // ==============================================
    // Browser Navigations Method
    // ==============================================

    pub fn browser_next(&mut self) {
        let i = match self.browser_state.selected() {
            Some(i) => {
                if i >= self.browser_items.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.browser_state.select(Some(i));
    }

    pub fn browser_prev(&mut self) {
        let i = match self.browser_state.selected() {
            Some(i) => {
                if i == 0 {
                    self.browser_items.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.browser_state.select(Some(i));
    }

    /// Enter selected directory or return PathBuf if it's a file
    pub fn browser_enter(&mut self) -> Option<PathBuf> {
        let i = self.browser_state.selected()?;
        let item = &self.browser_items.get(i)?;
        if item.is_dir {
            let new_path = item.path.clone();
            if let Ok(new_items) = browser::get_directory_content(&new_path) {
                self.current_dir = new_path;
                self.browser_items = new_items;
                self.browser_state.select(Some(0));
            }
            return None;
        } else {
            Some(item.path.clone())
        }
    }

    // ==============================================
    // Queue Navigation Methods
    // ==============================================

    pub fn queue_next(&mut self) {
        if self.queue.is_empty() {
            return;
        }
        let i = match self.queue_state.selected() {
            Some(i) => {
                if i >= self.queue.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.queue_state.select(Some(i));
    }

    pub fn queue_prev(&mut self) {
        if self.queue.is_empty() {
            return;
        }
        let i = match self.queue_state.selected() {
            Some(i) => {
                if i == 0 {
                    self.queue.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.queue_state.select(Some(i));
    }

    /// Get currently selected queue index
    pub fn queue_selected(&self) -> Option<usize> {
        self.queue_state.selected()
    }

    // ==============================================
    // Loop Mode Methods
    // ==============================================

    /// Cycle to the next loop mode
    pub fn next_loop_mode(&self) -> LoopMode {
        match self.loop_mode {
            LoopMode::Off => LoopMode::RepeatOne,
            LoopMode::RepeatOne => LoopMode::LoopAll,
            LoopMode::LoopAll => LoopMode::Shuffle,
            LoopMode::Shuffle => LoopMode::Off,
        }
    }

    // ==============================================
    // Dialog Methods
    // ==============================================

    /// Open the browser file dialog for a given path
    pub fn open_browser_dialog(&mut self, path: PathBuf) {
        self.browser_dialog = BrowserFileDialog::Open { path, selected: 0 };
    }

    /// Navigate dialog selection
    pub fn dialog_toggle(&mut self) {
        if let BrowserFileDialog::Open { selected, .. } = &mut self.browser_dialog {
            *selected = if *selected == 0 { 1 } else { 0 };
        }
    }

    /// Close the dialog
    pub fn close_dialog(&mut self) {
        self.browser_dialog = BrowserFileDialog::None;
    }

    /// Check if dialog is open
    pub fn is_dialog_open(&self) -> bool {
        !matches!(self.browser_dialog, BrowserFileDialog::None)
    }
}
