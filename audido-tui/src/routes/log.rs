use audido_core::engine::AudioEngineHandle;
use ratatui::{Frame, crossterm::event::KeyCode, layout::Rect, style::{Color, Modifier, Style}, widgets::{Block, Borders}};
use tui_logger::TuiLoggerWidget;

use crate::{router::{RouteAction, RouteHandler}, state::AppState};

/// Log route
#[derive(Debug, Clone)]
pub struct LogRoute;

impl RouteHandler for LogRoute {
    fn render(&self, frame: &mut Frame, area: Rect, state: &AppState) {
        draw_log_panel(frame, area, state);
    }

    fn handle_input(
        &mut self,
        key: KeyCode,
        _state: &mut AppState,
        _handle: &AudioEngineHandle,
    ) -> anyhow::Result<RouteAction> {
        match key {
            KeyCode::Up => {
                log::trace!("Log scroll up");
            }
            KeyCode::Down => {
                log::trace!("Log scroll down");
            }
            _ => {}
        }
        Ok(RouteAction::None)
    }

    fn name(&self) -> &str {
        "Log"
    }
}

/// Draw the log panel
fn draw_log_panel(f: &mut Frame, area: Rect, _state: &AppState) {
    // Panel is active when rendered (router-based system)
    let is_active = true;

    let border_style = if is_active {
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let log_widget = TuiLoggerWidget::default()
        .block(
            Block::default()
                .title(" 📋 Log ")
                .borders(Borders::ALL)
                .border_style(border_style),
        )
        .style(Style::default().fg(Color::White));

    f.render_widget(log_widget, area);
}

