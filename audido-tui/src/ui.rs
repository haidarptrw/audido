use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, Paragraph},
};
use tui_logger::TuiLoggerWidget;

use crate::state::{AppState, FocusedWidget};

/// Draw the TUI interface
pub fn draw(f: &mut Frame, state: &AppState) {
    // Main horizontal split: Playback (left) and Log (right)
    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .margin(1)
        .constraints([
            Constraint::Percentage(50), // Playback panel
            Constraint::Percentage(50), // Log panel
        ])
        .split(f.area());

    draw_playback_panel(f, main_chunks[0], state);
    draw_log_panel(f, main_chunks[1], state);
}

/// Draw the playback panel (left side)
fn draw_playback_panel(f: &mut Frame, area: Rect, state: &AppState) {
    let is_focused = state.focused_widget == FocusedWidget::Playback;

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(6), // Now playing info
            Constraint::Length(3), // Progress bar
            Constraint::Length(3), // Controls info
            Constraint::Min(0),    // Status/spacer
        ])
        .split(area);

    draw_now_playing(f, chunks[0], state, is_focused);
    draw_progress(f, chunks[1], state);
    draw_controls(f, chunks[2], state);
    draw_status(f, chunks[3], state);
}

/// Draw the log panel (right side)
fn draw_log_panel(f: &mut Frame, area: Rect, state: &AppState) {
    let is_focused = state.focused_widget == FocusedWidget::Log;

    let border_style = if is_focused {
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let log_widget = TuiLoggerWidget::default()
        .block(
            Block::default()
                .title(" 📋 Logs ")
                .borders(Borders::ALL)
                .border_style(border_style),
        )
        .style(Style::default().fg(Color::White));

    f.render_widget(log_widget, area);
}

/// Draw the now playing section
fn draw_now_playing(f: &mut Frame, area: Rect, state: &AppState, is_focused: bool) {
    let border_style = if is_focused {
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
    let controls = if state.focused_widget == FocusedWidget::Playback {
        vec![
            Span::styled("[Space]", Style::default().fg(Color::Yellow)),
            Span::raw(" Play/Pause  "),
            Span::styled("[S]", Style::default().fg(Color::Yellow)),
            Span::raw(" Stop  "),
            Span::styled("[←/→]", Style::default().fg(Color::Yellow)),
            Span::raw(" Seek  "),
            Span::styled("[↑/↓]", Style::default().fg(Color::Yellow)),
            Span::raw(" Volume  "),
            Span::styled("[Tab]", Style::default().fg(Color::Magenta)),
            Span::raw(" →Log  "),
            Span::styled("[Q]", Style::default().fg(Color::Red)),
            Span::raw(" Quit"),
        ]
    } else {
        vec![
            Span::styled("[↑/↓]", Style::default().fg(Color::Yellow)),
            Span::raw(" Scroll  "),
            Span::styled("[Tab]", Style::default().fg(Color::Magenta)),
            Span::raw(" →Playback  "),
            Span::styled("[Space]", Style::default().fg(Color::Yellow)),
            Span::raw(" Play/Pause  "),
            Span::styled("[Q]", Style::default().fg(Color::Red)),
            Span::raw(" Quit"),
        ]
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

    let volume_bar = format!("Volume: {:3.0}%", state.volume * 100.0);
    let focus_indicator = match state.focused_widget {
        FocusedWidget::Playback => "▶ Playback",
        FocusedWidget::Log => "📋 Log",
    };
    let status_text = format!(
        "{}  |  {}  |  Focus: {}",
        state.status_message, volume_bar, focus_indicator
    );

    let paragraph = Paragraph::new(status_text)
        .style(status_style)
        .block(Block::default().borders(Borders::ALL).title(" Status "));

    f.render_widget(paragraph, area);
}
