use audido_core::engine::AudioEngineHandle;
use ratatui::{Frame, crossterm::event::KeyCode, layout::Rect, style::{Color, Modifier, Style}, widgets::{Block, Borders, List, ListItem, Paragraph}};

use crate::{router::{RouteAction, RouteHandler}, state::AppState};

/// Queue route
#[derive(Debug, Clone)]
pub struct QueueRoute;

impl RouteHandler for QueueRoute {
    fn render(&self, frame: &mut Frame, area: Rect, state: &AppState) {
        draw_queue_panel(frame, area, state);
    }

    fn handle_input(
        &mut self,
        key: KeyCode,
        state: &mut AppState,
        handle: &AudioEngineHandle,
    ) -> anyhow::Result<RouteAction> {
        match key {
            KeyCode::Up => state.queue_prev(),
            KeyCode::Down => state.queue_next(),
            KeyCode::Enter => {
                if let Some(idx) = state.queue_selected() {
                    handle
                        .cmd_tx
                        .send(audido_core::commands::AudioCommand::PlayQueueIndex(idx))?;
                }
            }
            _ => {}
        }
        Ok(RouteAction::None)
    }

    fn name(&self) -> &str {
        "Queue"
    }
}


/// Draw the queue panel
pub fn draw_queue_panel(f: &mut Frame, area: Rect, state: &AppState) {
    // Panel is active when rendered (router-based system)
    let is_active = true;

    let title = format!(" Queue ({} tracks) ", state.queue.queue.len());
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(if is_active {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default()
        });

    let items: Vec<ListItem> = state
        .queue
        .queue
        .iter()
        .enumerate()
        .map(|(i, item)| {
            let is_current = state.queue.current_queue_index == Some(i);
            let prefix = if is_current { "▶ " } else { "  " };
            let name = item
                .metadata
                .as_ref()
                .and_then(|m| m.title.clone())
                .unwrap_or_else(|| {
                    item.path
                        .file_name()
                        .map(|s| s.to_string_lossy().to_string())
                        .unwrap_or_else(|| "Unknown".to_string())
                });
            let style = if is_current {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            ListItem::new(format!("{}{}", prefix, name)).style(style)
        })
        .collect();

    if items.is_empty() {
        let empty_msg = Paragraph::new("Queue is empty. Add files from Browser.")
            .style(Style::default().fg(Color::DarkGray))
            .block(block);
        f.render_widget(empty_msg, area);
    } else {
        let list = List::new(items)
            .block(block)
            .highlight_style(
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol(">> ");

        let mut list_state = state.queue.queue_state.clone();
        f.render_stateful_widget(list, area, &mut list_state);
    }
}
