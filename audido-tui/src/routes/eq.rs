use audido_core::{dsp::eq::Equalizer, engine::AudioEngineHandle};
use ratatui::{Frame, crossterm::event::KeyCode, layout::{Constraint, Direction, Layout, Rect}, style::{Color, Modifier, Style}, symbols, text::{Line, Span}, widgets::{Axis, Block, Borders, Chart, Clear, Dataset, GraphType, List, ListItem, Paragraph}};

use crate::{router::{RouteAction, RouteHandler}, state::{AppState, EqFocus, EqMode, SettingsOption}};

/// Equalizer route
#[derive(Debug, Clone)]
pub struct EqualizerRoute;

impl RouteHandler for EqualizerRoute {
    fn render(&self, frame: &mut Frame, area: Rect, state: &AppState) {
        draw_eq_panel(frame, area, state);
    }

    fn handle_input(
        &mut self,
        key: KeyCode,
        state: &mut AppState,
        handle: &AudioEngineHandle,
    ) -> anyhow::Result<RouteAction> {
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

    fn on_enter(&mut self, state: &mut AppState, _handle: &AudioEngineHandle) -> anyhow::Result<()> {
        state.eq_state.open_panel();
        Ok(())
    }

    fn on_exit(&mut self, state: &mut AppState, _handle: &AudioEngineHandle) -> anyhow::Result<()> {
        state.eq_state.close_panel();
        Ok(())
    }
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
        Span::styled("[T]", Style::default().fg(Color::Yellow)),
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

        // Determine if band panel is focused
        let is_band_focused = state.eq_state.eq_focus == EqFocus::BandPanel;
        let band_border_style = if is_band_focused {
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray)
        };

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
            Span::styled("[↑/↓]", Style::default().fg(Color::Yellow)),
            Span::raw(" Change Preset"),
        ]))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Preset ")
                .border_style(band_border_style),
        );

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

        // Determine if band panel is focused
        let is_band_focused = state.eq_state.eq_focus == EqFocus::BandPanel;
        let band_border_style = if is_band_focused {
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray)
        };

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
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(band_border_style)
                        .title(" Bands (↑↓ Select) "),
                )
        } else {
            let list = List::new(filter_items).block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(band_border_style)
                    .title(" Bands (↑↓ Select) "),
            );

            let mut list_state = ratatui::widgets::ListState::default();
            list_state.select(Some(state.eq_state.eq_selected_band));
            f.render_stateful_widget(list, chunks[0], &mut list_state);
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
        .map(|(name, value)| {
            Line::from(vec![
                Span::styled(format!("{}: ", name), Style::default().fg(Color::Gray)),
                Span::styled(value.clone(), Style::default().fg(Color::White)),
            ])
        })
        .collect();

    let paragraph =
        Paragraph::new(text).block(Block::default().borders(Borders::ALL).title(" Details "));
    f.render_widget(paragraph, area);
}

#[allow(dead_code)]
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
    let sample_rate = state.metadata.as_ref().map_or(44100, |m| m.sample_rate);
    let mut eq = Equalizer::new(sample_rate, state.eq_state.local_num_channels);
    eq.filters = state.eq_state.local_filters.clone();
    eq.master_gain = 10.0f32.powf(state.eq_state.local_master_gain / 20.0); // Convert dB to linear
    eq.parameters_changed();

    let width = 100;
    let data = eq.get_response_curve(100);

    
    // Transform to log scale for x-axis (frequency)
    // log10(20) ≈ 1.3, log10(20000) ≈ 4.3
    let data_points: Vec<(f64, f64)> = data
    .iter()
    .map(|(freq, db)| ((*freq as f64).log10(), *db as f64))
    .collect();

    // log::debug!("{:?}", data_points);

    let mut filter_curves: Vec<Vec<(f64, f64)>> = Vec::new();

    for filter in &state.eq_state.local_filters {
        let mut curve_points = Vec::with_capacity(width);
        
        // Generate points across the frequency spectrum for this single filter
        let start_freq: f32 = 20.0;
        let end_freq: f32 = 20000.0;
        let log_start = start_freq.ln();
        let log_end = end_freq.ln();
        let step = (log_end - log_start) / (width as f32 - 1.0);
        
        for i in 0..width {
            let log_f = log_start + step * i as f32;
            let f = log_f.exp();
            // Get magnitude of just this filter (no master gain)
            let db = filter.magnitude_db(f, sample_rate as f32);
            curve_points.push((f.log10() as f64, db as f64));
        }
        
        filter_curves.push(curve_points);
    }

    // Create filter center points for visualization (also in log scale)
    let filter_points: Vec<(f64, f64)> = state
        .eq_state
        .local_filters
        .iter()
        .map(|filter| {
            // Calculate the total response at the filter's center frequency
            // local_master_gain is already in dB, so use it directly
            let mut total_db = state.eq_state.local_master_gain;
            for flt in &state.eq_state.local_filters {
                total_db += flt.magnitude_db(filter.freq, sample_rate as f32);
            }
            ((filter.freq as f64).log10(), total_db as f64)
        })
        .collect();

    let datasets = vec![
        Dataset::default()
            .name("Response")
            .marker(symbols::Marker::Braille)
            .graph_type(GraphType::Line)
            .style(Style::default().fg(Color::Cyan))
            .data(&data_points),
        Dataset::default()
            .name("Filters")
            .marker(symbols::Marker::Dot)
            .graph_type(GraphType::Scatter)
            .style(Style::default().fg(Color::Yellow))
            .data(&filter_points),
    ];

    // Labels must be evenly spaced in log scale for proper alignment
    // 20 → 200 → 2000 → 20000 (each is 10x, so 1.0 apart in log10)
    let x_labels = vec![
        Span::styled("20", Style::default().fg(Color::Gray)),
        Span::styled("200", Style::default().fg(Color::Gray)),
        Span::styled("2k", Style::default().fg(Color::Gray)),
        Span::styled("20k", Style::default().fg(Color::Gray)),
    ];

    // Determine border style based on focus
    let is_focused = state.eq_state.eq_focus == EqFocus::CurvePanel;
    let border_style = if is_focused {
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let chart = Chart::new(datasets)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(border_style)
                .title(" Frequency Response (↑↓ Gain) "),
        )
        .x_axis(
            Axis::default()
                .title("Freq (Hz)")
                .bounds([20.0_f64.log10(), 20000.0_f64.log10()]) // ~1.3 to ~4.3
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
