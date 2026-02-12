use audido_core::{dsp::eq::Equalizer, engine::AudioEngineHandle};
use ratatui::{
    Frame,
    crossterm::event::KeyCode,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    symbols,
    text::{Line, Span},
    widgets::{Axis, Block, Borders, Chart, Clear, Dataset, GraphType, List, ListItem, Paragraph},
};

use crate::{
    router::{InterceptKeyResult, RouteAction, RouteHandler},
    state::AppState,
    states::{AudioState, EqFocus, EqMode, EqState, SettingsOption, eq::EqDialogState},
    ui::{draw_generic_dialog, open_modal},
};

/// Equalizer route
#[derive(Debug, Clone)]
pub struct EqualizerRoute;

impl EqualizerRoute {
    /// Handle input when the filter band config modal is open
    fn handle_filter_band_config_input(
        &mut self,
        key: KeyCode,
        state: &mut AppState,
        _handle: &AudioEngineHandle,
    ) -> anyhow::Result<RouteAction> {
        match key {
            KeyCode::Esc => {
                state.eq.close_filter_band_config();
            }
            // Future: add arrow keys / Enter to edit individual params here
            _ => {}
        }
        Ok(RouteAction::None)
    }

    /// Handle input when the band select dialog is open
    fn handle_band_select_dialog_input(
        &mut self,
        key: KeyCode,
        state: &mut AppState,
        _handle: &AudioEngineHandle,
    ) -> anyhow::Result<RouteAction> {
        match key {
            KeyCode::Up => {
                state.eq.prev();
            }
            KeyCode::Down => {
                state.eq.next();
            }
            KeyCode::Enter => {
                if let EqDialogState::EqBandSelect {
                    selected_band,
                    selected_dialog_option,
                } = state.eq.eq_dialog_state
                {
                    state.eq.close_filter_band_dialog();

                    match selected_dialog_option {
                        crate::states::eq::EqDialogOption::EditBand => {
                            state.eq.open_filter_band_config(selected_band);
                        }
                        crate::states::eq::EqDialogOption::ResetBand => {}
                    }
                }
            }
            KeyCode::Esc => {
                state.eq.close_filter_band_dialog();
            }
            _ => {}
        }
        Ok(RouteAction::None)
    }

    /// Handle input when no floating panel is open (default state)
    fn handle_default_input(
        &mut self,
        key: KeyCode,
        state: &mut AppState,
        handle: &AudioEngineHandle,
    ) -> anyhow::Result<RouteAction> {
        match key {
            KeyCode::Left | KeyCode::Right => {
                state.eq.toggle_focus();
            }
            KeyCode::Up => {
                match state.eq.eq_focus {
                    EqFocus::CurvePanel => {
                        state.eq.local_master_gain = (state.eq.local_master_gain + 0.5).min(12.0);
                        handle.cmd_tx.send(
                            audido_core::commands::AudioCommand::EqSetMasterGain(
                                state.eq.local_master_gain,
                            ),
                        )?;
                    }
                    EqFocus::BandPanel => {
                        match state.eq.eq_mode {
                            EqMode::Casual => {
                                // TODO: implement toggle preset
                            }
                            EqMode::Advanced => {
                                state.eq.prev();
                            }
                        }
                    }
                }
            }
            KeyCode::Down => match state.eq.eq_focus {
                EqFocus::CurvePanel => {
                    state.eq.local_master_gain = (state.eq.local_master_gain - 0.5).max(-12.0);
                    handle
                        .cmd_tx
                        .send(audido_core::commands::AudioCommand::EqSetMasterGain(
                            state.eq.local_master_gain,
                        ))?;
                }
                EqFocus::BandPanel => {
                    state.eq.next();
                }
            },
            KeyCode::Char('t') => {
                state.eq.toggle_enabled();
                handle
                    .cmd_tx
                    .send(audido_core::commands::AudioCommand::EqSetEnabled(
                        state.eq.eq_enabled,
                    ))?;
            }
            KeyCode::Char('m') => {
                state.eq.toggle_mode();
            }
            KeyCode::Char('a') => {
                if state.eq.local_filters.len() < 8 {
                    let new_id = state.eq.local_filters.len() as i16;
                    let new_filter = audido_core::dsp::eq::FilterNode::new(new_id, 1000.0);
                    state.eq.local_filters.push(new_filter);
                    handle
                        .cmd_tx
                        .send(audido_core::commands::AudioCommand::EqSetAllFilters(
                            state.eq.local_filters.clone(),
                        ))?;
                }
            }
            KeyCode::Enter => {
                if state.eq.eq_focus == EqFocus::BandPanel && state.eq.eq_mode == EqMode::Advanced {
                    state.eq.open_filter_band_dialog();
                }
            }
            _ => {}
        }
        Ok(RouteAction::None)
    }

    fn has_floating_panel(state: &AppState) -> bool {
        state.eq.eq_filter_band_config_opened.is_some()
            || state.eq.eq_dialog_state != EqDialogState::None
    }
}

impl RouteHandler for EqualizerRoute {
    fn render(&self, frame: &mut Frame, area: Rect, state: &AppState) {
        draw_eq_panel(frame, area, &state.eq, &state.audio);

        if let EqDialogState::EqBandSelect {
            selected_band,
            selected_dialog_option,
        } = &state.eq.eq_dialog_state
        {
            let title = format!(" Edit Band {} ", selected_band + 1);
            let props = crate::ui::DialogProperties {
                title: title.as_str(),
                options: vec!["Edit", "Reset Parameteres"],
                selected_index: selected_dialog_option.index(),
            };
            draw_generic_dialog(frame, area, props);
        }

        // draw filter band configuration modal
        if state.eq.eq_filter_band_config_opened.is_some() {
            open_modal(frame, area, &state.eq, draw_filter_band_config_modal);
        }
    }

    fn handle_input(
        &mut self,
        key: KeyCode,
        state: &mut AppState,
        handle: &AudioEngineHandle,
    ) -> anyhow::Result<RouteAction> {
        // State-first dispatch: route to the handler for the active UI context
        if state.eq.eq_filter_band_config_opened.is_some() {
            return self.handle_filter_band_config_input(key, state, handle);
        }

        if let EqDialogState::EqBandSelect { .. } = state.eq.eq_dialog_state {
            return self.handle_band_select_dialog_input(key, state, handle);
        }

        self.handle_default_input(key, state, handle)
    }

    fn name(&self) -> &str {
        "Equalizer"
    }

    fn on_enter(
        &mut self,
        state: &mut AppState,
        _handle: &AudioEngineHandle,
    ) -> anyhow::Result<()> {
        state.eq.open_panel();
        Ok(())
    }

    fn on_exit(&mut self, state: &mut AppState, _handle: &AudioEngineHandle) -> anyhow::Result<()> {
        state.eq.close_panel();
        Ok(())
    }

    fn intercept_global_key(
        &mut self,
        key: KeyCode,
        state: &mut AppState,
        handle: &AudioEngineHandle,
    ) -> InterceptKeyResult {
        // If any floating panel is open, intercept ALL keys and delegate to handle_input
        let has_floating_panel = EqualizerRoute::has_floating_panel(state);
        if has_floating_panel {
            // Delegate to handle_input which already does state-first dispatch
            let _ = self.handle_input(key, state, handle);
            return InterceptKeyResult::Handled;
        }
        InterceptKeyResult::Ignored
    }
}

pub fn draw_eq_panel(f: &mut Frame, area: Rect, eq_state: &EqState, audio_state: &AudioState) {
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

    draw_eq_mode_toggle(f, chunks[0], eq_state);
    draw_eq_graph(f, chunks[1], eq_state, audio_state);
    draw_eq_controls(f, chunks[2], eq_state);
}

fn draw_eq_mode_toggle(f: &mut Frame, area: Rect, eq_state: &EqState) {
    let is_casual = eq_state.eq_mode == EqMode::Casual;
    let is_enabled = eq_state.eq_enabled;

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

fn draw_eq_controls(f: &mut Frame, area: Rect, eq_state: &EqState) {
    let is_casual = eq_state.eq_mode == EqMode::Casual;

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
        let is_band_focused = eq_state.eq_focus == EqFocus::BandPanel;
        let band_border_style = if is_band_focused {
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        // Preset selector
        let preset_label = format!("{:?}", eq_state.local_preset);
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
        let gain_value = eq_state.local_master_gain;
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
        let is_band_focused = eq_state.eq_focus == EqFocus::BandPanel;
        let band_border_style = if is_band_focused {
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        // Filter band list
        let filter_items: Vec<ListItem> = eq_state
            .local_filters
            .iter()
            .enumerate()
            .map(|(i, filter)| {
                let is_selected = i == eq_state.eq_selected_band;
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
            list_state.select(Some(eq_state.eq_selected_band));
            f.render_stateful_widget(list, chunks[0], &mut list_state);
            return draw_filter_details(f, chunks[1], eq_state);
        };
        f.render_widget(filter_list, chunks[0]);

        // Empty details panel
        let details = Paragraph::new("Select a band to edit")
            .style(Style::default().fg(Color::DarkGray))
            .block(Block::default().borders(Borders::ALL).title(" Details "));
        f.render_widget(details, chunks[1]);
    }
}

fn draw_filter_details(f: &mut Frame, area: Rect, eq_state: &EqState) {
    if eq_state.local_filters.is_empty() {
        let details = Paragraph::new("No band selected")
            .style(Style::default().fg(Color::DarkGray))
            .block(Block::default().borders(Borders::ALL).title(" Details "));
        f.render_widget(details, area);
        return;
    }

    let filter = &eq_state.local_filters[eq_state.eq_selected_band];
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

fn draw_eq_graph(f: &mut Frame, area: Rect, eq_state: &EqState, audio_state: &AudioState) {
    // Create a temporary Equalizer to compute the response curve
    let sample_rate = audio_state
        .metadata
        .as_ref()
        .map_or(44100, |m| m.sample_rate);
    let mut eq = Equalizer::new(sample_rate, eq_state.local_num_channels);
    eq.filters = eq_state.local_filters.clone();
    eq.master_gain = (10.0f32).powf(eq_state.local_master_gain / 20.0); // Convert dB to linear
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

    for filter in &eq_state.local_filters {
        let mut curve_points = Vec::with_capacity(width);

        // Generate points across the frequency spectrum for this single filter
        let start_freq: f32 = 20.0;
        let end_freq: f32 = 20000.0;
        let log_start = start_freq.ln();
        let log_end = end_freq.ln();
        let step = (log_end - log_start) / ((width as f32) - 1.0);

        for i in 0..width {
            let log_f = log_start + step * (i as f32);
            let f = log_f.exp();
            // Get magnitude of just this filter (no master gain)
            let db = filter.magnitude_db(f, sample_rate as f32);
            curve_points.push((f.log10() as f64, db as f64));
        }

        filter_curves.push(curve_points);
    }

    // Create filter center points for visualization (also in log scale)
    let filter_points: Vec<(f64, f64)> = eq_state
        .local_filters
        .iter()
        .map(|filter| {
            // Calculate the total response at the filter's center frequency
            // local_master_gain is already in dB, so use it directly
            let mut total_db = eq_state.local_master_gain;
            for flt in &eq_state.local_filters {
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
    let is_focused = eq_state.eq_focus == EqFocus::CurvePanel;
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
                .bounds([(20.0_f64).log10(), (20000.0_f64).log10()]) // ~1.3 to ~4.3
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

fn draw_filter_band_config_modal(f: &mut Frame, area: Rect, eq_state: &EqState) {
    // Determine which band is being configured
    let band_index = match eq_state.eq_filter_band_config_opened {
        Some(idx) => idx,
        None => return,
    };

    let filter = match eq_state.local_filters.get(band_index) {
        Some(f) => f,
        None => return,
    };

    let title = format!(" Band {} Configuration ", band_index + 1);

    // Convert order to slope description (order * 6 dB/oct)
    let slope_label = format!(
        "{} dB/oct (order {})",
        filter.order as u16 * 6,
        filter.order
    );

    let params: Vec<(&str, String, Color)> = vec![
        ("Type", format!("{}", filter.filter_type), Color::Cyan),
        (
            "Frequency",
            format!("{} Hz", filter.freq as i32),
            Color::Green,
        ),
        (
            "Gain",
            format!("{:+.1} dB", filter.gain),
            if filter.gain >= 0.0 {
                Color::Green
            } else {
                Color::Red
            },
        ),
        ("Q Factor", format!("{:.3}", filter.q), Color::Yellow),
        ("Slope", slope_label, Color::Magenta),
    ];

    let text: Vec<Line> = params
        .iter()
        .map(|(label, value, color)| {
            Line::from(vec![
                Span::styled(
                    format!("  {:<12}", format!("{}:", label)),
                    Style::default().fg(Color::Gray),
                ),
                Span::styled(
                    value.clone(),
                    Style::default().fg(*color).add_modifier(Modifier::BOLD),
                ),
            ])
        })
        .collect();

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));

    let paragraph = Paragraph::new(text).block(block);
    f.render_widget(paragraph, area);
}
