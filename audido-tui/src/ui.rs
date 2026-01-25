use audido_core::{dsp::eq::Equalizer, queue::LoopMode};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    symbols,
    text::{Line, Span},
    widgets::{
        Axis, Block, Borders, Chart, Clear, Dataset, Gauge, GraphType, List, ListItem, Paragraph,
    },
};
use strum::IntoEnumIterator;
use tui_logger::TuiLoggerWidget;

use crate::state::{ActiveTab, AppState, BrowserFileDialog, EqFocus, EqMode, SettingsOption};

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

    // Draw dialog overlay if open
    if state.is_dialog_open() {
        draw_browser_dialog(f, f.area(), state);
    }
}

/// Draw the sidebar navigation
fn draw_sidebar(f: &mut Frame, area: Rect, state: &AppState, router: &crate::router::Router) {
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

/// Draw the playback panel
pub fn draw_playback_panel(f: &mut Frame, area: Rect, state: &AppState) {
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
pub fn draw_log_panel(f: &mut Frame, area: Rect, state: &AppState) {
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

pub fn draw_browser_panel(f: &mut Frame, area: Rect, state: &AppState) {
    let is_active = state.active_tab == ActiveTab::Browser;

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
fn draw_controls(f: &mut Frame, area: Rect, state: &AppState, router: &crate::router::Router) {
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
                Span::styled("[Enter]", Style::default().fg(Color::Yellow)),
                Span::raw(" Toggle EQ  "),
                Span::styled("[M]", Style::default().fg(Color::Yellow)),
                Span::raw(" Mode  "),
                Span::styled("[↑/↓]", Style::default().fg(Color::Yellow)),
                Span::raw(" Adjust Gain  "),
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
pub fn draw_queue_panel(f: &mut Frame, area: Rect, state: &AppState) {
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
    if let BrowserFileDialog::Open { path, selected } = &state.browser.dialog {
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

pub fn draw_settings_panel(f: &mut Frame, area: Rect, state: &AppState) {
    let is_active = state.active_tab == ActiveTab::Settings;

    // If EQ panel is open, split area for settings list and EQ panel
    if state.eq_state.show_eq {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(35), // Settings list
                Constraint::Percentage(65), // EQ Panel
            ])
            .split(area);

        draw_settings_list(f, chunks[0], state, is_active);
        draw_eq_panel(f, chunks[1], state);
    } else {
        draw_settings_list(f, area, state, is_active);
    }
}

fn draw_settings_list(f: &mut Frame, area: Rect, state: &AppState, is_active: bool) {
    let block = Block::default()
        .title(" Settings ")
        .borders(Borders::ALL)
        .border_style(if is_active {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::DarkGray)
        });

    let inner = block.inner(area);
    f.render_widget(block, area);

    let items: Vec<ListItem> = state
        .settings_state
        .items
        .iter()
        .enumerate()
        .map(|(i, setting)| {
            let is_selected =
                state.settings_state.selected_index == i && !state.settings_state.is_dialog_open;

            let value_str = match setting {
                SettingsOption::Equalizer => {
                    if state.eq_state.eq_enabled {
                        "On"
                    } else {
                        "Off"
                    }
                }
            };

            let prefix = if is_selected { "▶ " } else { "  " };
            let style = if is_selected {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };

            ListItem::new(Line::from(vec![
                Span::styled(format!("{}{}", prefix, setting.label()), style),
                Span::raw(" "),
                Span::styled(format!("[{}]", value_str), Style::default().fg(Color::Cyan)),
            ]))
        })
        .collect();

    let list = List::new(items);
    f.render_widget(list, inner);
}

pub fn draw_eq_panel(f: &mut Frame, area: Rect, state: &AppState) {
    let block = Block::default()
        .title(" Equalizer ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Magenta));

    let inner = block.inner(area);
    f.render_widget(block, area);

    // Split EQ panel: Mode Toggle, EQ Graph, and Controls
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Mode toggle row
            Constraint::Min(8),    // EQ Graph
            Constraint::Length(6), // EQ Controls
        ])
        .split(inner);

    draw_eq_mode_toggle(f, chunks[0], state);
    draw_eq_graph(f, chunks[1], state);
    draw_eq_controls(f, chunks[2], state);
}

fn draw_eq_mode_toggle(f: &mut Frame, area: Rect, state: &AppState) {
    let is_casual = state.eq_state.eq_mode == EqMode::Casual;
    let is_enabled = state.eq_state.eq_enabled;

    let enabled_style = if is_enabled {
        Style::default()
            .fg(Color::Green)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
    };

    let casual_style = if is_casual {
        Style::default()
            .fg(Color::Green)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let advanced_style = if !is_casual {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let mode_line = Line::from(vec![
        Span::styled("EQ: ", Style::default().fg(Color::White)),
        Span::styled(if is_enabled { "ON" } else { "OFF" }, enabled_style),
        Span::raw("  │  "),
        Span::styled("Mode: ", Style::default().fg(Color::White)),
        Span::styled(if is_casual { "● " } else { "○ " }, casual_style),
        Span::styled("Casual", casual_style),
        Span::raw("  "),
        Span::styled(if !is_casual { "● " } else { "○ " }, advanced_style),
        Span::styled("Advanced", advanced_style),
        Span::raw("  │  "),
        Span::styled("[Enter]", Style::default().fg(Color::Yellow)),
        Span::raw(" Toggle EQ  "),
        Span::styled("[M]", Style::default().fg(Color::Yellow)),
        Span::raw(" Mode"),
    ]);

    let paragraph = Paragraph::new(mode_line).block(Block::default().borders(Borders::BOTTOM));
    f.render_widget(paragraph, area);
}

fn draw_eq_controls(f: &mut Frame, area: Rect, state: &AppState) {
    let is_casual = state.eq_state.eq_mode == EqMode::Casual;

    if is_casual {
        // Casual mode: Show preset selector and master gain
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(50), // Preset
                Constraint::Percentage(50), // Master Gain
            ])
            .split(area);

        // Preset selector
        let preset_label = format!("{:?}", state.eq_state.local_preset);
        let preset_paragraph = Paragraph::new(Line::from(vec![
            Span::styled("Preset: ", Style::default().fg(Color::Gray)),
            Span::styled(
                preset_label,
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("\n"),
            Span::styled("[←/→]", Style::default().fg(Color::Yellow)),
            Span::raw(" Change Preset"),
        ]))
        .block(Block::default().borders(Borders::ALL).title(" Preset "));
        f.render_widget(preset_paragraph, chunks[0]);

        // Master gain
        let gain_value = state.eq_state.local_master_gain;
        let gain_display = if gain_value >= 0.0 {
            format!("+{:.1} dB", gain_value)
        } else {
            format!("{:.1} dB", gain_value)
        };
        let gain_paragraph = Paragraph::new(Line::from(vec![
            Span::styled("Master: ", Style::default().fg(Color::Gray)),
            Span::styled(
                gain_display,
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("\n"),
            Span::styled("[↑/↓]", Style::default().fg(Color::Yellow)),
            Span::raw(" Adjust Gain"),
        ]))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Master Gain "),
        );
        f.render_widget(gain_paragraph, chunks[1]);
    } else {
        // Advanced mode: Show filter bands with editable parameters
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(60), // Filter list
                Constraint::Percentage(40), // Selected filter details
            ])
            .split(area);

        // Filter band list
        let filter_items: Vec<ListItem> = state
            .eq_state
            .local_filters
            .iter()
            .enumerate()
            .map(|(i, filter)| {
                let is_selected = i == state.eq_state.eq_selected_band;
                let prefix = if is_selected { "▶ " } else { "  " };
                let style = if is_selected {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                };
                let filter_info = format!(
                    "{}Band {}: {:?} @ {}Hz",
                    prefix,
                    i + 1,
                    filter.filter_type,
                    filter.freq as i32
                );
                ListItem::new(filter_info).style(style)
            })
            .collect();

        let filter_list = if filter_items.is_empty() {
            Paragraph::new("No filters. Press [A] to add.")
                .style(Style::default().fg(Color::DarkGray))
                .block(Block::default().borders(Borders::ALL).title(" Bands "))
        } else {
            let list = List::new(filter_items)
                .block(Block::default().borders(Borders::ALL).title(" Bands "));
            f.render_widget(list, chunks[0]);
            return draw_filter_details(f, chunks[1], state);
        };
        f.render_widget(filter_list, chunks[0]);

        // Empty details panel
        let details = Paragraph::new("Select a band to edit")
            .style(Style::default().fg(Color::DarkGray))
            .block(Block::default().borders(Borders::ALL).title(" Details "));
        f.render_widget(details, chunks[1]);
    }
}

fn draw_filter_details(f: &mut Frame, area: Rect, state: &AppState) {
    if state.eq_state.local_filters.is_empty() {
        let details = Paragraph::new("No band selected")
            .style(Style::default().fg(Color::DarkGray))
            .block(Block::default().borders(Borders::ALL).title(" Details "));
        f.render_widget(details, area);
        return;
    }

    let filter = &state.eq_state.local_filters[state.eq_state.eq_selected_band];
    let params = [
        ("Type", format!("{:?}", filter.filter_type)),
        ("Freq", format!("{} Hz", filter.freq as i32)),
        ("Gain", format!("{:+.1} dB", filter.gain)),
        ("Q", format!("{:.2}", filter.q)),
    ];

    let text: Vec<Line> = params
        .iter()
        .enumerate()
        .map(|(i, (name, value))| {
            let is_selected = i == state.eq_state.eq_selected_param
                && state.eq_state.eq_focus == EqFocus::EditParam;
            let style = if is_selected {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            Line::from(vec![
                Span::styled(format!("{}: ", name), Style::default().fg(Color::Gray)),
                Span::styled(value.clone(), style),
            ])
        })
        .collect();

    let paragraph =
        Paragraph::new(text).block(Block::default().borders(Borders::ALL).title(" Details "));
    f.render_widget(paragraph, area);
}

fn draw_settings_dialog(f: &mut Frame, area: Rect, state: &AppState) {
    let selected_setting = state.settings_state.items[state.settings_state.selected_index];

    let choices = match selected_setting {
        SettingsOption::Equalizer => {
            vec!["Enable", "Disable"]
        }
    };

    let width = 30;
    let height: u16 = choices.len() as u16 + 4;
    let x = (area.width.saturating_sub(width)) / 2;
    let y = (area.height.saturating_sub(height)) / 2;
    let dialog_area = Rect::new(x, y, width, height);

    f.render_widget(Clear, dialog_area);

    let block = Block::default()
        .title(format!(" {} ", selected_setting.label()))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));

    let inner = block.inner(dialog_area);
    f.render_widget(block, dialog_area);

    let choices_items: Vec<ListItem> = choices
        .iter()
        .enumerate()
        .map(|(i, choice)| {
            let is_selected = i == state.settings_state.dialog_selection_index;
            let prefix = if is_selected { "● " } else { "○ " };
            let style = if is_selected {
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Gray)
            };
            ListItem::new(Span::styled(format!("{}{}", prefix, choice), style))
        })
        .collect();

    let list = List::new(choices_items);
    f.render_widget(list, inner);
}

fn draw_eq_graph(f: &mut Frame, area: Rect, state: &AppState) {
    // Create a temporary Equalizer to compute the response curve
    let mut eq = Equalizer::new(44100, state.eq_state.local_num_channels);
    eq.filters = state.eq_state.local_filters.clone();
    eq.master_gain = 10.0f32.powf(state.eq_state.local_master_gain / 20.0); // Convert dB to linear
    eq.parameters_changed();

    let data = eq.get_response_curve(100);

    // Create Dataset
    let data_points: Vec<(f64, f64)> = data.iter().map(|f| (f.0 as f64, f.1 as f64)).collect();
    let datasets = vec![
        Dataset::default()
            .name("Response")
            .marker(symbols::Marker::Braille)
            .graph_type(GraphType::Line)
            .style(Style::default().fg(Color::Cyan))
            .data(&data_points),
    ];

    let x_labels = vec![
        Span::styled("20", Style::default().fg(Color::Gray)),
        Span::styled("100", Style::default().fg(Color::Gray)),
        Span::styled("1k", Style::default().fg(Color::Gray)),
        Span::styled("10k", Style::default().fg(Color::Gray)),
        Span::styled("20k", Style::default().fg(Color::Gray)),
    ];

    let chart = Chart::new(datasets)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Frequency Response "),
        )
        .x_axis(
            Axis::default()
                .title("Freq (Hz)")
                .bounds([20.0, 20000.0])
                .labels(x_labels),
        )
        .y_axis(
            Axis::default()
                .title("Gain (dB)")
                .bounds([-18.0, 18.0])
                .labels(vec![Span::raw("-18"), Span::raw("0"), Span::raw("+18")]),
        );

    f.render_widget(chart, area);
}
