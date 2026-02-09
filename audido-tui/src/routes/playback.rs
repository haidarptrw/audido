use audido_core::{commands::AudioCommand, engine::AudioEngineHandle};
use ratatui::{Frame, crossterm::event::KeyCode, layout::{Constraint, Direction, Layout, Rect}, style::{Color, Modifier, Style}, text::{Line, Span}, widgets::{Block, Borders, Gauge, Paragraph}};

use crate::{router::{RouteAction, RouteHandler}, state::AppState};

// ==================================================================
// Playback Route Implementation
// ==================================================================

#[derive(Debug, Clone)]
pub struct PlaybackRoute;

impl RouteHandler for PlaybackRoute {
    fn render(&self, frame: &mut Frame, area: Rect, state: &AppState) {
        draw_playback_panel(frame, area, state);
    }

    fn handle_input(
        &mut self,
        key: KeyCode,
        state: &mut AppState,
        handle: &AudioEngineHandle,
    ) -> anyhow::Result<RouteAction> {
        match key {
            KeyCode::Up => {
                state.volume = (state.volume + 0.1).min(1.0);
                handle
                    .cmd_tx
                    .send(AudioCommand::SetVolume(state.volume))?;
            }
            KeyCode::Down => {
                state.volume = (state.volume - 0.1).max(0.0);
                handle
                    .cmd_tx
                    .send(AudioCommand::SetVolume(state.volume))?;
            }
            KeyCode::Right => {
                let new_pos = state.position + 5.0;
                handle
                    .cmd_tx
                    .send(AudioCommand::Seek(new_pos))?;
            }
            KeyCode::Left => {
                let new_pos = (state.position - 5.0).max(0.0);
                handle
                    .cmd_tx
                    .send(AudioCommand::Seek(new_pos))?;
            }
            KeyCode::Char(' ') => {
                if state.is_playing {
                    handle.cmd_tx.send(AudioCommand::Pause)?;
                } else {
                    handle.cmd_tx.send(AudioCommand::Play)?;
                }
            }
            KeyCode::Char('s') => {
                handle.cmd_tx.send(AudioCommand::Stop)?;
            }
            KeyCode::Char('n') => {
                handle.cmd_tx.send(AudioCommand::Next)?;
            }
            KeyCode::Char('p') => {
                handle.cmd_tx.send(AudioCommand::Previous)?;
            }
            KeyCode::Char('l') => {
                let next_mode = state.next_loop_mode();
                handle.cmd_tx.send(AudioCommand::SetLoopMode(next_mode))?;
            }
            _ => {}
        }
        Ok(RouteAction::None)
    }

    fn name(&self) -> &str {
        "Playback"
    }
}

/// Draw the playback panel
pub fn draw_playback_panel(f: &mut Frame, area: Rect, state: &AppState) {
    // Panel is active when rendered (router-based system)
    let is_active = true;

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(6), // Now playing info
            Constraint::Length(3), // Progress bar
            Constraint::Length(3), // Controls info
            Constraint::Min(0),    // Status/spacer
        ])
        .split(area);

    draw_now_playing(f, chunks[0], state, is_active);
    draw_progress(f, chunks[1], state);
}

/// Draw the now playing section
fn draw_now_playing(f: &mut Frame, area: Rect, state: &AppState, is_active: bool) {
    let border_style = if is_active {
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let block = Block::default()
        .title(" 🎵 Now Playing ")
        .borders(Borders::ALL)
        .border_style(border_style);

    let inner = block.inner(area);
    f.render_widget(block, area);

    if let Some(ref metadata) = state.metadata {
        let title = metadata.title.as_deref().unwrap_or("Unknown Title");
        let artist = metadata.author.as_deref().unwrap_or("Unknown Artist");
        let album = metadata.album.as_deref().unwrap_or("Unknown Album");

        let text = vec![
            Line::from(vec![Span::styled(
                title,
                Style::default().fg(Color::White).bold(),
            )]),
            Line::from(vec![Span::styled(artist, Style::default().fg(Color::Gray))]),
            Line::from(vec![Span::styled(
                album,
                Style::default().fg(Color::DarkGray),
            )]),
        ];

        let paragraph = Paragraph::new(text);
        f.render_widget(paragraph, inner);
    } else {
        let text = Paragraph::new("No audio loaded").style(Style::default().fg(Color::DarkGray));
        f.render_widget(text, inner);
    }
}

/// Draw the progress bar
fn draw_progress(f: &mut Frame, area: Rect, state: &AppState) {
    let progress_pct = (state.progress() * 100.0) as u16;
    let position_str = AppState::format_time(state.position);
    let duration_str = AppState::format_time(state.duration);

    let label = format!("{} / {}", position_str, duration_str);

    let gauge = Gauge::default()
        .block(Block::default().borders(Borders::ALL))
        .gauge_style(Style::default().fg(Color::Cyan).bg(Color::DarkGray))
        .percent(progress_pct)
        .label(label);

    f.render_widget(gauge, area);
}

// ==================================================================
// Metadata Route Implementation
// ==================================================================

// #[derive(Debug, Clone)]
// pub struct SongMetadataRoute;

// impl RouteHandler for SongMetadataRoute {
//     fn render(&self, frame: &mut ratatui::Frame, area: ratatui::prelude::Rect, state: &crate::state::AppState) {
//         todo!()
//     }

//     fn handle_input(
//         &mut self,
//         key: ratatui::crossterm::event::KeyCode,
//         state: &mut crate::state::AppState,
//         handle: &audido_core::engine::AudioEngineHandle,
//     ) -> anyhow::Result<crate::router::RouteAction> {
//         todo!()
//     }

//     fn name(&self) -> &str {
//         todo!()
//     }
// }