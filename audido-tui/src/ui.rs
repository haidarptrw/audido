use audido_core::queue::LoopMode;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

use crate::state::AppState;

/// Draw the TUI interface
pub fn draw(f: &mut Frame, state: &AppState, router: &crate::router::Router) {
    // Main horizontal split: Sidebar (left) and Main Content (right)
    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .margin(1)
        .constraints([
            Constraint::Length(15), // Sidebar navigation
            Constraint::Min(40),    // Main content area
        ])
        .split(f.area());

    draw_sidebar(f, main_chunks[0], state, router);
    draw_main_content(f, main_chunks[1], state, router);
}

/// Draw the sidebar navigation
fn draw_sidebar(f: &mut Frame, area: Rect, _state: &AppState, router: &crate::router::Router) {
    let block = Block::default()
        .title(" Navigation ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(area);
    f.render_widget(block, area);

    // Navigation items - generated from router tab names
    let current_route_name = router.current().name();
    let nav_text: Vec<Line> = crate::router::tab_names()
        .iter()
        .map(|tab_name| {
            let is_active = *tab_name == current_route_name;
            let prefix = if is_active { "▶ " } else { "  " };
            let style = if is_active {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Gray)
            };
            Line::from(Span::styled(format!("{}{}", prefix, tab_name), style))
        })
        .collect();

    let paragraph = Paragraph::new(nav_text);
    f.render_widget(paragraph, inner);
}

/// Draw the main content area based on active route
fn draw_main_content(f: &mut Frame, area: Rect, state: &AppState, router: &crate::router::Router) {
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

    // Draw the specific panel via the router
    router.current().render(f, content_area, state);

    // Draw global footers on every tab
    draw_controls(f, controls_area, state, router);
    draw_status(f, status_area, state);
}

/// Draw the controls help section
fn draw_controls(f: &mut Frame, area: Rect, _state: &AppState, router: &crate::router::Router) {
    let route_name = router.current().name();
    let controls = match route_name {
        "Playback" => {
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
        "Queue" => {
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
        "Log" => {
            vec![
                Span::styled("[↑/↓]", Style::default().fg(Color::Yellow)),
                Span::raw(" Scroll  "),
                Span::styled("[Tab]", Style::default().fg(Color::Magenta)),
                Span::raw(" Switch Tab  "),
                Span::styled("[Q]", Style::default().fg(Color::Red)),
                Span::raw(" Quit"),
            ]
        }
        "Browser" | "File Options" => {
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
        "Settings" => {
            vec![
                Span::styled("[↑/↓]", Style::default().fg(Color::Yellow)),
                Span::raw(" Navigate  "),
                Span::styled("[Enter]", Style::default().fg(Color::Yellow)),
                Span::raw(" Select  "),
                Span::styled("[Tab]", Style::default().fg(Color::Magenta)),
                Span::raw(" Switch Tab  "),
                Span::styled("[Q]", Style::default().fg(Color::Red)),
                Span::raw(" Quit"),
            ]
        }
        "Equalizer" => {
            vec![
                Span::styled("[←/→]", Style::default().fg(Color::Yellow)),
                Span::raw(" Focus  "),
                Span::styled("[T]", Style::default().fg(Color::Yellow)),
                Span::raw(" Toggle  "),
                Span::styled("[M]", Style::default().fg(Color::Yellow)),
                Span::raw(" Mode  "),
                Span::styled("[A]", Style::default().fg(Color::Yellow)),
                Span::raw(" Add  "),
                Span::styled("[Esc]", Style::default().fg(Color::Yellow)),
                Span::raw(" Back  "),
                Span::styled("[Q]", Style::default().fg(Color::Red)),
                Span::raw(" Quit"),
            ]
        }
        _ => {
            vec![
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
    let status_style = if state.audio.error_message.is_some() {
        Style::default().fg(Color::Red)
    } else if state.audio.is_playing {
        Style::default().fg(Color::Green)
    } else {
        Style::default().fg(Color::Yellow)
    };

    let loop_icon = match state.queue.loop_mode {
        LoopMode::Off => "➡️ Off",
        LoopMode::RepeatOne => "🔂 One",
        LoopMode::LoopAll => "🔁 All",
        LoopMode::Shuffle => "🔀 Shuffle",
    };

    let volume_bar = format!("Vol: {:3.0}%", state.audio.volume * 100.0);
    let queue_info = format!("Queue: {}", state.queue.queue.len());
    let status_text = format!(
        "{}  |  {}  |  {}  |  {}",
        state.audio.status_message, volume_bar, queue_info, loop_icon
    );

    let paragraph = Paragraph::new(status_text)
        .style(status_style)
        .block(Block::default().borders(Borders::ALL).title(" Status "));

    f.render_widget(paragraph, area);
}
