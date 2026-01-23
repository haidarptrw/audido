use audido_core::queue::LoopMode;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Gauge, List, ListItem, Paragraph},
};
use strum::IntoEnumIterator;
use tui_logger::TuiLoggerWidget;

use crate::state::{ActiveTab, AppState, BrowserFileDialog};

/// Draw the TUI interface
pub fn draw(f: &mut Frame, state: &AppState) {
    // Main horizontal split: Sidebar (left) and Main Content (right)
    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .margin(1)
        .constraints([
            Constraint::Length(15), // Sidebar navigation
            Constraint::Min(40),    // Main content area
        ])
        .split(f.area());

    draw_sidebar(f, main_chunks[0], state);
    draw_main_content(f, main_chunks[1], state);

    // Draw dialog overlay if open
    if state.is_dialog_open() {
        draw_browser_dialog(f, f.area(), state);
    }
}

/// Draw the sidebar navigation
fn draw_sidebar(f: &mut Frame, area: Rect, state: &AppState) {
    let block = Block::default()
        .title(" Navigation ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(area);
    f.render_widget(block, area);

    // Navigation items - generated from ActiveTab enum
    let nav_text: Vec<Line> = ActiveTab::iter()
        .map(|tab| {
            let is_active = state.active_tab == tab;
            let prefix = if is_active { "▶ " } else { "  " };
            let style = if is_active {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Gray)
            };
            Line::from(Span::styled(format!("{}{}", prefix, tab), style))
        })
        .collect();

    let paragraph = Paragraph::new(nav_text);
    f.render_widget(paragraph, inner);
}

/// Draw the main content area based on active tab
fn draw_main_content(f: &mut Frame, area: Rect, state: &AppState) {
    // Split the main area into Content (top) and Footer (bottom)
    // Footer contains Controls (3 lines) and Status (3 lines)
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(0),    // Panel specific content
            Constraint::Length(3), // Controls info
            Constraint::Length(3), // Status bar
        ])
        .split(area);

    let content_area = chunks[0];
    let controls_area = chunks[1];
    let status_area = chunks[2];

    // Draw the specific panel in the top content area
    match state.active_tab {
        ActiveTab::Playback => draw_playback_panel(f, content_area, state),
        ActiveTab::Queue => draw_queue_panel(f, content_area, state),
        ActiveTab::Log => draw_log_panel(f, content_area, state),
        ActiveTab::Browser => draw_browser_panel(f, content_area, state),
    }

    // Draw global footers on every tab
    draw_controls(f, controls_area, state);
    draw_status(f, status_area, state);
}

/// Draw the playback panel
fn draw_playback_panel(f: &mut Frame, area: Rect, state: &AppState) {
    let is_active = state.active_tab == ActiveTab::Playback;

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

/// Draw the log panel
fn draw_log_panel(f: &mut Frame, area: Rect, state: &AppState) {
    let is_active = state.active_tab == ActiveTab::Log;

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

fn draw_browser_panel(f: &mut Frame, area: Rect, state: &AppState) {
    let is_active = state.active_tab == ActiveTab::Browser;

    // Title shows current path
    let title = if state.current_dir.as_os_str().is_empty() {
        " Browser: System Drives ".to_string()
    } else {
        format!(" Browser: {} ", state.current_dir.to_string_lossy())
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
        .browser_items
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
    let mut list_state = state.browser_state.clone();
    f.render_stateful_widget(list, area, &mut list_state);
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

/// Draw the controls help section
fn draw_controls(f: &mut Frame, area: Rect, state: &AppState) {
    let controls = match state.active_tab {
        ActiveTab::Playback => {
            vec![
                Span::styled("[Space]", Style::default().fg(Color::Yellow)),
                Span::raw(" Play/Pause  "),
                Span::styled("[N/P]", Style::default().fg(Color::Yellow)),
                Span::raw(" Next/Prev  "),
                Span::styled("[L]", Style::default().fg(Color::Yellow)),
                Span::raw(" Loop  "),
                Span::styled("[←/→]", Style::default().fg(Color::Yellow)),
                Span::raw(" Seek  "),
                Span::styled("[Tab]", Style::default().fg(Color::Magenta)),
                Span::raw(" Switch Tab  "),
                Span::styled("[Q]", Style::default().fg(Color::Red)),
                Span::raw(" Quit"),
            ]
        }
        ActiveTab::Queue => {
            vec![
                Span::styled("[↑/↓]", Style::default().fg(Color::Yellow)),
                Span::raw(" Navigate  "),
                Span::styled("[Enter]", Style::default().fg(Color::Yellow)),
                Span::raw(" Play  "),
                Span::styled("[N/P]", Style::default().fg(Color::Yellow)),
                Span::raw(" Next/Prev  "),
                Span::styled("[L]", Style::default().fg(Color::Yellow)),
                Span::raw(" Loop  "),
                Span::styled("[Tab]", Style::default().fg(Color::Magenta)),
                Span::raw(" Switch Tab  "),
                Span::styled("[Q]", Style::default().fg(Color::Red)),
                Span::raw(" Quit"),
            ]
        }
        ActiveTab::Log => {
            vec![
                Span::styled("[↑/↓]", Style::default().fg(Color::Yellow)),
                Span::raw(" Scroll  "),
                Span::styled("[Tab]", Style::default().fg(Color::Magenta)),
                Span::raw(" Switch Tab  "),
                Span::styled("[Q]", Style::default().fg(Color::Red)),
                Span::raw(" Quit"),
            ]
        }
        ActiveTab::Browser => {
            vec![
                Span::styled("[↑/↓]", Style::default().fg(Color::Yellow)),
                Span::raw(" Nav  "),
                Span::styled("[Enter]", Style::default().fg(Color::Yellow)),
                Span::raw(" Select  "),
                Span::styled("[Tab]", Style::default().fg(Color::Magenta)),
                Span::raw(" Switch Tab  "),
                Span::styled("[Q]", Style::default().fg(Color::Red)),
                Span::raw(" Quit"),
            ]
        }
    };

    let paragraph = Paragraph::new(Line::from(controls))
        .block(Block::default().borders(Borders::ALL).title(" Controls "));

    f.render_widget(paragraph, area);
}

/// Draw the status section
fn draw_status(f: &mut Frame, area: Rect, state: &AppState) {
    let status_style = if state.error_message.is_some() {
        Style::default().fg(Color::Red)
    } else if state.is_playing {
        Style::default().fg(Color::Green)
    } else {
        Style::default().fg(Color::Yellow)
    };

    let loop_icon = match state.loop_mode {
        LoopMode::Off => "➡️ Off",
        LoopMode::RepeatOne => "🔂 One",
        LoopMode::LoopAll => "🔁 All",
        LoopMode::Shuffle => "🔀 Shuffle",
    };

    let volume_bar = format!("Vol: {:3.0}%", state.volume * 100.0);
    let queue_info = format!("Queue: {}", state.queue.len());
    let status_text = format!(
        "{}  |  {}  |  {}  |  {}",
        state.status_message, volume_bar, queue_info, loop_icon
    );

    let paragraph = Paragraph::new(status_text)
        .style(status_style)
        .block(Block::default().borders(Borders::ALL).title(" Status "));

    f.render_widget(paragraph, area);
}

/// Draw the queue panel
fn draw_queue_panel(f: &mut Frame, area: Rect, state: &AppState) {
    let is_active = state.active_tab == ActiveTab::Queue;

    let title = format!(" Queue ({} tracks) ", state.queue.len());
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
        .iter()
        .enumerate()
        .map(|(i, item)| {
            let is_current = state.current_queue_index == Some(i);
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

        let mut list_state = state.queue_state.clone();
        f.render_stateful_widget(list, area, &mut list_state);
    }
}

/// Draw the browser file dialog overlay
fn draw_browser_dialog(f: &mut Frame, area: Rect, state: &AppState) {
    if let BrowserFileDialog::Open { path, selected } = &state.browser_dialog {
        // Calculate centered dialog area
        let dialog_width = 40;
        let dialog_height = 8;
        let x = (area.width.saturating_sub(dialog_width)) / 2;
        let y = (area.height.saturating_sub(dialog_height)) / 2;
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
