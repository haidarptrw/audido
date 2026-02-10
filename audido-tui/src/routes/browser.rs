use audido_core::engine::AudioEngineHandle;
use ratatui::{
    crossterm::event::KeyCode,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
    Frame,
};

use crate::{
    router::{RouteAction, RouteHandler},
    routes::playback::PlaybackRoute,
    state::AppState,
    states::BrowserFileDialog,
};

/// Browser route - handles both browsing and file dialog as internal state
#[derive(Debug, Clone)]
pub struct BrowserRoute;

impl RouteHandler for BrowserRoute {
    fn render(&self, frame: &mut Frame, area: Rect, state: &AppState) {
        draw_browser_panel(frame, area, state);

        // Draw dialog overlay if open
        if state.browser_state.is_dialog_open() {
            draw_browser_dialog(frame, area, state);
        }
    }

    fn handle_input(
        &mut self,
        key: KeyCode,
        state: &mut AppState,
        handle: &AudioEngineHandle,
    ) -> anyhow::Result<RouteAction> {
        // Check if dialog is open - handle dialog input
        if state.browser_state.is_dialog_open() {
            match key {
                KeyCode::Up | KeyCode::Down => {
                    state.browser_state.dialog_toggle();
                }
                KeyCode::Enter => {
                    if let BrowserFileDialog::Open { path, selected } = &state.browser_state.dialog
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
                            state.browser_state.close_dialog();
                            // Navigate to playback
                            return Ok(RouteAction::Replace(Box::new(PlaybackRoute)));
                        } else {
                            // Add to Queue
                            handle
                                .cmd_tx
                                .send(audido_core::commands::AudioCommand::AddToQueue(vec![
                                    path_str,
                                ]))?;
                            state.browser_state.close_dialog();
                        }
                    }
                }
                KeyCode::Esc => {
                    state.browser_state.close_dialog();
                }
                _ => {}
            }
        } else {
            // Normal browser navigation
            match key {
                KeyCode::Up => state.browser_state.prev(),
                KeyCode::Down => state.browser_state.next(),
                KeyCode::Enter => {
                    if let Some(path) = state.browser_state.enter() {
                        // Open dialog as internal state
                        state.browser_state.open_dialog(path);
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
    let title = if state.browser_state.current_dir.as_os_str().is_empty() {
        " Browser: System Drives ".to_string()
    } else {
        format!(" Browser: {} ", state.browser_state.current_dir.to_string_lossy())
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
        .browser_state
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
    let mut list_state = state.browser_state.list_state.clone();
    f.render_stateful_widget(list, area, &mut list_state);
}

fn draw_browser_dialog(f: &mut Frame, area: Rect, state: &AppState) {
    if let BrowserFileDialog::Open { path, selected } = &state.browser_state.dialog {
        // Calculate centered dialog area within the given region
        let dialog_width = 40;
        let dialog_height = 8;
        let x = area.x + (area.width.saturating_sub(dialog_width)) / 2;
        let y = area.y + (area.height.saturating_sub(dialog_height)) / 2;
        let dialog_area = Rect::new(x, y, dialog_width, dialog_height);

        // Clear the area behind dialog
        f.render_widget(Clear, dialog_area);

        let filename = path
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "file".to_string());

        let block = Block::default()
            .title(format!(" {} ", filename))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow));

        let inner = block.inner(dialog_area);
        f.render_widget(block, dialog_area);

        let options = vec![
            ("▶ Play Now", *selected == 0),
            ("+ Add to Queue", *selected == 1),
        ];

        let text: Vec<Line> = options
            .iter()
            .map(|(label, is_selected)| {
                let style = if *is_selected {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::Gray)
                };
                let prefix = if *is_selected { "> " } else { "  " };
                Line::from(Span::styled(format!("{}{}", prefix, label), style))
            })
            .collect();

        let paragraph = Paragraph::new(text);
        f.render_widget(paragraph, inner);
    }
}
