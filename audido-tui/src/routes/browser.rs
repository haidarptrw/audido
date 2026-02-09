use audido_core::engine::AudioEngineHandle;
use ratatui::{Frame, crossterm::event::KeyCode, layout::Rect, style::{Color, Modifier, Style}, text::{Line, Span}, widgets::{Block, Borders, List, ListItem}};

use crate::{router::{RouteAction, RouteHandler}, routes::playback::PlaybackRoute, state::AppState};

/// Browser route - handles both browsing and file dialog as internal state
#[derive(Debug, Clone)]
pub struct BrowserRoute;

impl RouteHandler for BrowserRoute {
    fn render(&self, frame: &mut Frame, area: Rect, state: &AppState) {
        draw_browser_panel(frame, area, state);
    }

    fn handle_input(
        &mut self,
        key: KeyCode,
        state: &mut AppState,
        handle: &AudioEngineHandle,
    ) -> anyhow::Result<RouteAction> {
        // Check if dialog is open - handle dialog input
        if state.browser.is_dialog_open() {
            match key {
                KeyCode::Up | KeyCode::Down => {
                    state.browser.dialog_toggle();
                }
                KeyCode::Enter => {
                    if let crate::state::BrowserFileDialog::Open { path, selected } =
                        &state.browser.dialog
                    {
                        let path_str = path.to_string_lossy().to_string();

                        if *selected == 0 {
                            // Play Now
                            handle
                                .cmd_tx
                                .send(audido_core::commands::AudioCommand::ClearQueue)?;
                            handle
                                .cmd_tx
                                .send(audido_core::commands::AudioCommand::AddToQueue(vec![
                                    path_str,
                                ]))?;
                            handle
                                .cmd_tx
                                .send(audido_core::commands::AudioCommand::PlayQueueIndex(0))?;
                            state.browser.close_dialog();
                            // Navigate to playback
                            return Ok(RouteAction::Replace(Box::new(PlaybackRoute)));
                        } else {
                            // Add to Queue
                            handle
                                .cmd_tx
                                .send(audido_core::commands::AudioCommand::AddToQueue(vec![
                                    path_str,
                                ]))?;
                            state.browser.close_dialog();
                        }
                    }
                }
                KeyCode::Esc => {
                    state.browser.close_dialog();
                }
                _ => {}
            }
        } else {
            // Normal browser navigation
            match key {
                KeyCode::Up => state.browser.prev(),
                KeyCode::Down => state.browser.next(),
                KeyCode::Enter => {
                    if let Some(path) = state.browser.enter() {
                        // Open dialog as internal state
                        state.browser.open_dialog(path);
                    }
                }
                _ => {}
            }
        }
        Ok(RouteAction::None)
    }

    fn name(&self) -> &str {
        "Browser"
    }
}

pub fn draw_browser_panel(f: &mut Frame, area: Rect, state: &AppState) {
    // Panel is active when rendered (router-based system)
    let is_active = true;

    // Title shows current path
    let title = if state.browser.current_dir.as_os_str().is_empty() {
        " Browser: System Drives ".to_string()
    } else {
        format!(" Browser: {} ", state.browser.current_dir.to_string_lossy())
    };

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(if is_active {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default()
        });

    let items: Vec<ListItem> = state
        .browser
        .items
        .iter()
        .map(|item| {
            let icon = if item.is_dir { "📁" } else { "🎵" };
            let color = if item.is_dir {
                Color::Blue
            } else {
                Color::White
            };

            ListItem::new(Line::from(vec![
                Span::styled(format!("{} ", icon), Style::default().fg(color)),
                Span::raw(&item.name),
            ]))
        })
        .collect();

    let list = List::new(items)
        .block(block)
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(">> ");

    // We must clone the state to pass mutable reference to render_stateful_widget
    // But since we can't mutate state here, we pass a clone. Ratatui uses this for offset calculation.
    let mut list_state = state.browser.list_state.clone();
    f.render_stateful_widget(list, area, &mut list_state);
}
