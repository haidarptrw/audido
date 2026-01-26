use std::path::PathBuf;

use audido_core::{
    browser::{self, FileEntry},
    commands::AudioResponse,
    dsp::eq::{EqPreset, FilterNode},
    metadata::AudioMetadata,
    queue::{LoopMode, QueueItem},
};
use ratatui::widgets::ListState;

/// Dialog shown when selecting a file in browser
#[derive(Debug, Clone, Default)]
pub enum BrowserFileDialog {
    #[default]
    None,
    /// Dialog open with path and selected option (0=Play Now, 1=Add to Queue)
    Open { path: PathBuf, selected: usize },
}

/// Browser state for file navigation
#[derive(Debug, Clone)]
pub struct BrowserState {
    pub current_dir: PathBuf,
    pub items: Vec<FileEntry>,
    pub list_state: ListState,
    pub dialog: BrowserFileDialog,
}

impl BrowserState {
    pub fn new() -> Self {
        let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let items = browser::get_directory_content(&current_dir).unwrap_or_default();
        let mut list_state = ListState::default();
        if !items.is_empty() {
            list_state.select(Some(0));
        }

        Self {
            current_dir,
            items,
            list_state,
            dialog: BrowserFileDialog::None,
        }
    }

    pub fn next(&mut self) {
        let i = match self.list_state.selected() {
            Some(i) => {
                if i >= self.items.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    pub fn prev(&mut self) {
        let i = match self.list_state.selected() {
            Some(i) => {
                if i == 0 {
                    self.items.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    /// Enter selected directory or return PathBuf if it's a file
    pub fn enter(&mut self) -> Option<PathBuf> {
        let i = self.list_state.selected()?;
        let item = &self.items.get(i)?;
        if item.is_dir {
            let new_path = item.path.clone();
            if let Ok(new_items) = browser::get_directory_content(&new_path) {
                self.current_dir = new_path;
                self.items = new_items;
                self.list_state.select(Some(0));
            }
            return None;
        } else {
            Some(item.path.clone())
        }
    }

    /// Open the browser file dialog for a given path
    pub fn open_dialog(&mut self, path: PathBuf) {
        self.dialog = BrowserFileDialog::Open { path, selected: 0 };
    }

    /// Navigate dialog selection
    pub fn dialog_toggle(&mut self) {
        if let BrowserFileDialog::Open { selected, .. } = &mut self.dialog {
            *selected = if *selected == 0 { 1 } else { 0 };
        }
    }

    /// Close the dialog
    pub fn close_dialog(&mut self) {
        self.dialog = BrowserFileDialog::None;
    }

    /// Check if dialog is open
    pub fn is_dialog_open(&self) -> bool {
        !matches!(self.dialog, BrowserFileDialog::None)
    }
}

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum EqMode {
    Casual,
    Advanced,
}

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum EqFocus {
    /// Curve/Graph panel - up/down controls master gain
    CurvePanel,
    /// Band panel - up/down selects bands (Advanced mode only)
    BandPanel,
}

#[derive(Debug, Clone)]
pub struct EqState {
    pub show_eq: bool,
    pub eq_enabled: bool,
    pub eq_mode: EqMode,
    pub eq_focus: EqFocus,
    /// Index of the selected filter node
    pub eq_selected_band: usize,
    pub eq_selected_param: usize,
    // Local copy of filters for immediate UI feedback before sending to Engine
    pub local_filters: Vec<FilterNode>,
    pub local_preset: EqPreset,
    pub local_master_gain: f32,
    pub local_num_channels: u16,
}

impl EqState {
    pub fn new() -> Self {
        Self {
            show_eq: false,
            eq_enabled: false,
            eq_mode: EqMode::Casual,
            eq_focus: EqFocus::CurvePanel,
            eq_selected_band: 0,
            eq_selected_param: 0,
            local_filters: EqPreset::default().set_filters(),
            local_preset: EqPreset::default(),
            local_master_gain: 0.0,
            local_num_channels: 2, // Default to stereo
        }
    }

    /// Toggle EQ enabled state
    pub fn toggle_enabled(&mut self) {
        self.eq_enabled = !self.eq_enabled;
    }

    /// Toggle between Casual and Advanced mode
    pub fn toggle_mode(&mut self) {
        self.eq_mode = match self.eq_mode {
            EqMode::Casual => EqMode::Advanced,
            EqMode::Advanced => EqMode::Casual,
        };
    }

    /// Toggle focus between CurvePanel and BandPanel
    pub fn toggle_focus(&mut self) {
        self.eq_focus = match self.eq_focus {
            EqFocus::CurvePanel => EqFocus::BandPanel,
            EqFocus::BandPanel => EqFocus::CurvePanel,
        };
    }

    /// Select next band in the filter list
    pub fn next_band(&mut self) {
        if !self.local_filters.is_empty() {
            self.eq_selected_band = (self.eq_selected_band + 1) % self.local_filters.len();
        }
    }

    /// Select previous band in the filter list
    pub fn prev_band(&mut self) {
        if !self.local_filters.is_empty() {
            self.eq_selected_band = if self.eq_selected_band == 0 {
                self.local_filters.len() - 1
            } else {
                self.eq_selected_band - 1
            };
        }
    }

    /// Open EQ panel
    pub fn open_panel(&mut self) {
        self.show_eq = true;
    }

    /// Close EQ panel
    pub fn close_panel(&mut self) {
        self.show_eq = false;
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SettingsOption {
    Equalizer,
}

impl SettingsOption {
    pub fn label(&self) -> &str {
        match self {
            SettingsOption::Equalizer => "Equalizer",
        }
    }
}

#[derive(Debug, Clone)]
pub struct SettingsState {
    pub items: Vec<SettingsOption>,
    pub selected_index: usize,
    /// Is the choice dialog currently open?
    pub is_dialog_open: bool,
    /// Selection index inside the dialog (e.g., 0=On, 1=Off)
    pub dialog_selection_index: usize,
}

impl SettingsState {
    pub fn new() -> Self {
        Self {
            items: vec![SettingsOption::Equalizer],
            selected_index: 0,
            is_dialog_open: false,
            dialog_selection_index: 0,
        }
    }

    pub fn next_item(&mut self) {
        if !self.items.is_empty() {
            self.selected_index = (self.selected_index + 1) % self.items.len();
        }
    }

    pub fn prev_item(&mut self) {
        if !self.items.is_empty() {
            self.selected_index = (self.selected_index + self.items.len() - 1) % self.items.len();
        }
    }

    #[allow(dead_code)]
    pub fn open_dialog(&mut self) {
        self.is_dialog_open = true;
        self.dialog_selection_index = 0;
    }

    #[allow(dead_code)]
    pub fn close_dialog(&mut self) {
        self.is_dialog_open = false;
    }

    #[allow(dead_code)]
    pub fn next_dialog(&mut self, choice_count: usize) {
        if choice_count > 0 {
            self.dialog_selection_index = (self.dialog_selection_index + 1) % choice_count;
        }
    }

    #[allow(dead_code)]
    pub fn prev_dialog(&mut self, choice_count: usize) {
        if choice_count > 0 {
            self.dialog_selection_index =
                (self.dialog_selection_index + choice_count - 1) % choice_count;
        }
    }
}

/// Application state for the TUI
pub struct AppState {
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
    pub browser: BrowserState,

    // ==============================
    // Queue State
    // ==============================
    pub queue: Vec<QueueItem>,
    pub current_queue_index: Option<usize>,
    pub loop_mode: LoopMode,
    pub queue_state: ListState,

    // EQ State
    pub eq_state: EqState,

    // Settings State
    pub settings_state: SettingsState,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            is_playing: false,
            position: 0.0,
            duration: 0.0,
            volume: 1.0,
            metadata: None,
            status_message: "No audio loaded. Pass a file path as argument.".to_string(),
            error_message: None,
            browser: BrowserState::new(),

            // Queue State
            queue: Vec::new(),
            current_queue_index: None,
            loop_mode: LoopMode::Off,
            queue_state: ListState::default(),

            // Other State
            eq_state: EqState::new(),
            settings_state: SettingsState::new(),
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
    // Dialog Methods (delegated to browser)
    // ==============================================

    /// Check if dialog is open (convenience delegate)
    pub fn is_dialog_open(&self) -> bool {
        self.browser.is_dialog_open()
    }
}
