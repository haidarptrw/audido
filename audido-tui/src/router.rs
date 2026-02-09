use anyhow::Result;
use ratatui::{Frame, layout::Rect};

use crate::{routes::{browser::BrowserRoute, log::LogRoute, playback::PlaybackRoute, queue::QueueRoute}, state::AppState, ui};
use audido_core::{engine::AudioEngineHandle};
use ratatui::crossterm::event::KeyCode;

/// Trait that all routes must implement
/// This enables dynamic dispatch and polymorphic behavior
pub trait RouteHandler: std::fmt::Debug {
    /// Render this route's UI
    fn render(&self, frame: &mut Frame, area: Rect, state: &AppState);

    /// Handle keyboard input for this route
    /// Returns Ok(RouteAction) to indicate what should happen next
    fn handle_input(
        &mut self,
        key: KeyCode,
        state: &mut AppState,
        handle: &AudioEngineHandle,
    ) -> anyhow::Result<RouteAction>;

    /// Get the display name for breadcrumbs/navigation
    fn name(&self) -> &str;

    /// Optional: Called when entering this route
    fn on_enter(&mut self, _state: &mut AppState, _handle: &AudioEngineHandle) -> Result<()> {
        Ok(())
    }

    /// Optional: Called when leaving this route
    fn on_exit(&mut self, _state: &mut AppState, _handle: &AudioEngineHandle) -> Result<()> {
        Ok(())
    }

    /// Optional: Check if this route can be exited (for validation)
    fn can_exit(&self, _state: &AppState) -> bool {
        true
    }

    fn help_items(&self, _state: &AppState) -> Vec<(&str, &str)> {
        vec![
            ("Tab", "Switch Tab"),
            ("Q", "Quit"),
        ]
    }
}

/// Actions that can be returned from route handlers
#[derive(Debug)]
#[allow(dead_code)]
pub enum RouteAction {
    /// Do nothing, stay on current route
    None,
    /// Go back to previous route
    Pop,
    /// Navigate to a new route
    Push(Box<dyn RouteHandler>),
    /// Replace current route with a new one
    Replace(Box<dyn RouteHandler>),
    /// Clear stack and navigate to route
    Reset(Box<dyn RouteHandler>),
    /// Quit the application
    Quit,
}

/// Router manages the navigation stack
pub struct Router {
    /// Stack of route handlers, last element is current route
    stack: Vec<Box<dyn RouteHandler>>,
}

impl Router {
    pub fn new(initial_route: Box<dyn RouteHandler>) -> Self {
        Self {
            stack: vec![initial_route],
        }
    }

    /// Get current route (top of stack)
    pub fn current(&self) -> &dyn RouteHandler {
        self.stack
            .last()
            .expect("Stack should never be empty")
            .as_ref()
    }

    /// Get mutable reference to current route
    pub fn current_mut(&mut self) -> &mut Box<dyn RouteHandler> {
        self.stack.last_mut().expect("Stack should never be empty")
    }

    /// Execute a route action
    pub fn execute_action(
        &mut self,
        action: RouteAction,
        state: &mut AppState,
        handle: &AudioEngineHandle,
    ) -> Result<bool> {
        match action {
            RouteAction::None => Ok(false),
            RouteAction::Pop => {
                self.pop(state, handle)?;
                Ok(false)
            }
            RouteAction::Push(route) => {
                self.push(route, state, handle)?;
                Ok(false)
            }
            RouteAction::Replace(route) => {
                self.replace(route, state, handle)?;
                Ok(false)
            }
            RouteAction::Reset(route) => {
                self.reset_to(route, state, handle)?;
                Ok(false)
            }
            RouteAction::Quit => Ok(true),
        }
    }

    /// Navigate to a new route (push onto stack)
    pub fn push(
        &mut self,
        mut route: Box<dyn RouteHandler>,
        state: &mut AppState,
        handle: &AudioEngineHandle,
    ) -> Result<()> {
        route.on_enter(state, handle)?;
        self.stack.push(route);
        Ok(())
    }

    /// Go back (pop from stack)
    pub fn pop(
        &mut self,
        state: &mut AppState,
        handle: &AudioEngineHandle,
    ) -> Result<Option<Box<dyn RouteHandler>>> {
        // Keep at least one route in the stack
        if self.stack.len() > 1 {
            if let Some(current) = self.stack.last() {
                if !current.can_exit(state) {
                    return Ok(None);
                }
            }

            if let Some(mut route) = self.stack.pop() {
                route.on_exit(state, handle)?;
                return Ok(Some(route));
            }
        }
        Ok(None)
    }

    /// Replace current route (useful for tab switching)
    pub fn replace(
        &mut self,
        mut new_route: Box<dyn RouteHandler>,
        state: &mut AppState,
        handle: &AudioEngineHandle,
    ) -> Result<()> {
        if let Some(mut old_route) = self.stack.pop() {
            old_route.on_exit(state, handle)?;
        }
        new_route.on_enter(state, handle)?;
        self.stack.push(new_route);
        Ok(())
    }

    /// Clear stack and navigate to route
    pub fn reset_to(
        &mut self,
        mut route: Box<dyn RouteHandler>,
        state: &mut AppState,
        handle: &AudioEngineHandle,
    ) -> Result<()> {
        // Exit all routes
        while let Some(mut old_route) = self.stack.pop() {
            old_route.on_exit(state, handle)?;
        }
        route.on_enter(state, handle)?;
        self.stack.push(route);
        Ok(())
    }

    /// Get the depth of navigation
    pub fn depth(&self) -> usize {
        self.stack.len()
    }
}

/// Get a route handler for a given tab name
pub fn route_for_name(name: &str) -> Box<dyn RouteHandler> {
    match name {
        "Playback" => Box::new(PlaybackRoute),
        "Queue" => Box::new(QueueRoute),
        "Browser" => Box::new(BrowserRoute),
        "Settings" => Box::new(SettingsRoute),
        "Log" => Box::new(LogRoute),
        _ => Box::new(PlaybackRoute),
    }
}

/// Get all main tab names in order
pub fn tab_names() -> &'static [&'static str] {
    &["Playback", "Queue", "Browser", "Settings", "Log"]
}

// ============================================================================
// Concrete Route Implementations
// ============================================================================

/// Settings route
#[derive(Debug, Clone)]
pub struct SettingsRoute;

impl RouteHandler for SettingsRoute {
    fn render(&self, frame: &mut Frame, area: Rect, state: &AppState) {
        crate::ui::draw_settings_panel(frame, area, state);
    }

    fn handle_input(
        &mut self,
        key: KeyCode,
        state: &mut AppState,
        _handle: &AudioEngineHandle,
    ) -> Result<RouteAction> {
        match key {
            KeyCode::Up => state.settings_state.prev_item(),
            KeyCode::Down => state.settings_state.next_item(),
            KeyCode::Enter => {
                // Navigate to EQ panel
                return Ok(RouteAction::Push(Box::new(EqualizerRoute)));
            }
            _ => {}
        }
        Ok(RouteAction::None)
    }

    fn name(&self) -> &str {
        "Settings"
    }
}

/// Equalizer route
#[derive(Debug, Clone)]
pub struct EqualizerRoute;

impl RouteHandler for EqualizerRoute {
    fn render(&self, frame: &mut Frame, area: Rect, state: &AppState) {
        crate::ui::draw_eq_panel(frame, area, state);
    }

    fn handle_input(
        &mut self,
        key: KeyCode,
        state: &mut AppState,
        handle: &AudioEngineHandle,
    ) -> Result<RouteAction> {
        match key {
            KeyCode::Left | KeyCode::Right => {
                // Toggle focus between curve panel and band panel
                state.eq_state.toggle_focus();
            }
            KeyCode::Up => {
                match state.eq_state.eq_focus {
                    crate::state::EqFocus::CurvePanel => {
                        // Increase master gain
                        state.eq_state.local_master_gain =
                            (state.eq_state.local_master_gain + 0.5).min(12.0);
                        handle.cmd_tx.send(
                            audido_core::commands::AudioCommand::EqSetMasterGain(
                                state.eq_state.local_master_gain,
                            ),
                        )?;
                    }
                    crate::state::EqFocus::BandPanel => {
                        // Select previous band (in Advanced mode)
                        match state.eq_state.eq_mode {
                            crate::state::EqMode::Casual => {
                                // state.eq_state.prev_band();
                                // TODO: implement toggle preset
                            }
                            crate::state::EqMode::Advanced => {
                                state.eq_state.prev_band();
                            }
                        }
                    }
                }
            }
            KeyCode::Down => {
                match state.eq_state.eq_focus {
                    crate::state::EqFocus::CurvePanel => {
                        // Decrease master gain
                        state.eq_state.local_master_gain =
                            (state.eq_state.local_master_gain - 0.5).max(-12.0);
                        handle.cmd_tx.send(
                            audido_core::commands::AudioCommand::EqSetMasterGain(
                                state.eq_state.local_master_gain,
                            ),
                        )?;
                    }
                    crate::state::EqFocus::BandPanel => {
                        // Select next band (in Advanced mode)
                        state.eq_state.next_band();
                    }
                }
            }
            KeyCode::Char('t') => {
                state.eq_state.toggle_enabled();
                handle
                    .cmd_tx
                    .send(audido_core::commands::AudioCommand::EqSetEnabled(
                        state.eq_state.eq_enabled,
                    ))?;
            }
            KeyCode::Enter => {
                match state.eq_state.eq_focus {
                    crate::state::EqFocus::CurvePanel => {}
                    crate::state::EqFocus::BandPanel => {
                        // TODO: implement a small modal to modify the filter band parameters
                    }
                }
            }
            KeyCode::Char('m') => {
                state.eq_state.toggle_mode();
            }
            KeyCode::Char('a') => {
                if state.eq_state.local_filters.len() < 8 {
                    let new_id = state.eq_state.local_filters.len() as i16;
                    let new_filter = audido_core::dsp::eq::FilterNode::new(new_id, 1000.0);
                    state.eq_state.local_filters.push(new_filter);
                    handle
                        .cmd_tx
                        .send(audido_core::commands::AudioCommand::EqSetAllFilters(
                            state.eq_state.local_filters.clone(),
                        ))?;
                }
            }
            _ => {}
        }
        Ok(RouteAction::None)
    }

    fn name(&self) -> &str {
        "Equalizer"
    }

    fn on_enter(&mut self, state: &mut AppState, _handle: &AudioEngineHandle) -> Result<()> {
        state.eq_state.open_panel();
        Ok(())
    }

    fn on_exit(&mut self, state: &mut AppState, _handle: &AudioEngineHandle) -> Result<()> {
        state.eq_state.close_panel();
        Ok(())
    }
}
