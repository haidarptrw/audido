use audido_core::dsp::eq::{EqPreset, FilterNode};

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

