use std::fs::canonicalize;
use std::time::Duration;
use std::{io, path::PathBuf};

use audido_core::browser;
use audido_core::dsp::eq::FilterNode;
use ratatui::{
    backend::CrosstermBackend,
    crossterm::{
        event::{self, Event, KeyCode, KeyEventKind},
        execute,
        terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
    },
};

use audido_core::{
    commands::AudioCommand,
    engine::{AudioEngine, AudioEngineHandle},
};

mod macros;
mod state;
mod ui;

use state::{ActiveTab, AppState, BrowserFileDialog};

fn main() -> anyhow::Result<()> {
    // Initialize tui_logger for TUI log display
    tui_logger::init_logger(log::LevelFilter::Debug).expect("Failed to init tui_logger");
    tui_logger::set_default_level(log::LevelFilter::Debug);

    log::info!("Starting Audido TUI");

    // Get audio file paths from command line args
    let args: Vec<String> = std::env::args().skip(1).collect();

    // Create audio engine and get communication handle
    let (engine, handle) = AudioEngine::new()?;

    // Spawn audio engine on dedicated thread
    let _engine_thread = engine.spawn();

    // Run TUI
    let result = run_tui(handle, args);

    // Ensure clean shutdown
    result
}

fn run_tui(handle: AudioEngineHandle, initial_files: Vec<String>) -> anyhow::Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = ratatui::Terminal::new(backend)?;

    let mut state = AppState::new();

    // Handle initial setup (Browser context & Queue loading)
    setup_initial_state(&mut state, &handle, initial_files)?;

    loop {
        // Handle audio engine responses
        while let Ok(response) = handle.resp_rx.try_recv() {
            state.handle_response(response);
        }

        // Draw UI
        terminal.draw(|f| ui::draw(f, &state))?;

        // Handle input
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    // Returns true if 'q' is pressed to break the loop
                    if handle_key_event(key.code, &mut state, &handle)? {
                        break;
                    }
                }
            }
        }
    }

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}

// --- Initialization Logic ---

fn setup_initial_state(
    state: &mut AppState,
    handle: &AudioEngineHandle,
    files: Vec<String>,
) -> anyhow::Result<()> {
    if files.is_empty() {
        return Ok(());
    }

    // 1. Set Browser Context based on the first file
    if let Some(first_file) = files.first() {
        let path = PathBuf::from(first_file);

        let target_dir = if let Ok(abs_path) = canonicalize(&path) {
            if abs_path.is_dir() {
                Some(abs_path)
            } else {
                abs_path.parent().map(|p| p.to_path_buf())
            }
        } else {
            if path.is_dir() {
                Some(path)
            } else {
                path.parent().map(|p| p.to_path_buf())
            }
        };

        if let Some(dir) = target_dir {
            if let Ok(items) = browser::get_directory_content(&dir) {
                state.browser.current_dir = dir;
                state.browser.items = items;
                state.browser.list_state.select(Some(0));
                log::info!("Browser context set to: {:?}", state.browser.current_dir);
            }
        }
    }

    // 2. Load Queue
    log::info!("Adding {} files to queue from CLI", files.len());
    handle.cmd_tx.send(AudioCommand::AddToQueue(files))?;
    handle.cmd_tx.send(AudioCommand::PlayQueueIndex(0))?;
    state.status_message = "Loading queue...".to_string();

    Ok(())
}

fn any(_: &AppState) -> bool {
    true
}
fn dialog_open(s: &AppState) -> bool {
    s.is_dialog_open()
}
fn no_dialog(s: &AppState) -> bool {
    !s.is_dialog_open()
}
fn in_playback(s: &AppState) -> bool {
    !s.is_dialog_open() && s.active_tab == ActiveTab::Playback
}
fn in_browser(s: &AppState) -> bool {
    !s.is_dialog_open() && s.active_tab == ActiveTab::Browser
}
fn in_queue(s: &AppState) -> bool {
    !s.is_dialog_open() && s.active_tab == ActiveTab::Queue
}
fn in_log(s: &AppState) -> bool {
    !s.is_dialog_open() && s.active_tab == ActiveTab::Log
}
fn in_settings(s: &AppState) -> bool {
    !s.is_dialog_open() && s.active_tab == ActiveTab::Settings
}

fn handle_key_event(
    key: KeyCode,
    state: &mut AppState,
    handle: &AudioEngineHandle,
) -> anyhow::Result<bool> {
    handlers!(state, handle, key => {
        // === Global / Media Keys ===

        fn quit(KeyCode::Char('q'), any) {
            let _ = handle.cmd_tx.send(AudioCommand::Quit);
            return Ok(true); // Stop loop
        }

        fn toggle_playback(KeyCode::Char(' '), any) {
            if state.is_playing {
                handle.cmd_tx.send(AudioCommand::Pause)?;
            } else {
                handle.cmd_tx.send(AudioCommand::Play)?;
            }
        }

        fn stop(KeyCode::Char('s'), any) {
            handle.cmd_tx.send(AudioCommand::Stop)?;
        }

        fn next_track(KeyCode::Char('n'), any) {
            handle.cmd_tx.send(AudioCommand::Next)?;
        }

        fn prev_track(KeyCode::Char('p'), any) {
            handle.cmd_tx.send(AudioCommand::Previous)?;
        }

        fn toggle_loop(KeyCode::Char('l'), any) {
            let next_mode = state.next_loop_mode();
            handle.cmd_tx.send(AudioCommand::SetLoopMode(next_mode))?;
        }

        // === Navigation ===

        fn next_tab(KeyCode::Tab, no_dialog) {
            state.next_tab();
        }

        fn close_dialog(KeyCode::Esc, dialog_open) {
            state.browser.close_dialog();
        }

        // === Dialog Controls ===

        fn dialog_up(KeyCode::Up, dialog_open) {
            state.browser.dialog_toggle();
        }

        fn dialog_down(KeyCode::Down, dialog_open) {
            state.browser.dialog_toggle();
        }

        fn dialog_enter(KeyCode::Enter, dialog_open) {
            if let BrowserFileDialog::Open { path, selected } = &state.browser.dialog {
                let path_str = path.to_string_lossy().to_string();

                if *selected == 0 { // Play Now
                    handle.cmd_tx.send(AudioCommand::ClearQueue)?;
                    handle.cmd_tx.send(AudioCommand::AddToQueue(vec![path_str]))?;
                    handle.cmd_tx.send(AudioCommand::PlayQueueIndex(0))?;
                    state.active_tab = ActiveTab::Playback;
                } else { // Add to Queue
                    handle.cmd_tx.send(AudioCommand::AddToQueue(vec![path_str]))?;
                }
            }
            state.browser.close_dialog();
        }

        // === Playback Tab ===

        fn volume_up(KeyCode::Up, in_playback) {
            state.volume = (state.volume + 0.1).min(1.0);
            handle.cmd_tx.send(AudioCommand::SetVolume(state.volume))?;
        }

        fn volume_down(KeyCode::Down, in_playback) {
            state.volume = (state.volume - 0.1).max(0.0);
            handle.cmd_tx.send(AudioCommand::SetVolume(state.volume))?;
        }

        fn seek_forward(KeyCode::Right, in_playback) {
            let new_pos = state.position + 5.0;
            handle.cmd_tx.send(AudioCommand::Seek(new_pos))?;
        }

        fn seek_backward(KeyCode::Left, in_playback) {
            let new_pos = (state.position - 5.0).max(0.0);
            handle.cmd_tx.send(AudioCommand::Seek(new_pos))?;
        }

        // === Browser Tab ===

        fn browser_up(KeyCode::Up, in_browser) {
            state.browser.prev();
        }

        fn browser_down(KeyCode::Down, in_browser) {
            state.browser.next();
        }

        fn browser_enter(KeyCode::Enter, in_browser) {
            if let Some(path) = state.browser.enter() {
                state.browser.open_dialog(path);
            }
        }

        // === Queue Tab ===

        fn queue_up(KeyCode::Up, in_queue) {
            state.queue_prev();
        }

        fn queue_down(KeyCode::Down, in_queue) {
            state.queue_next();
        }

        fn queue_enter(KeyCode::Enter, in_queue) {
            if let Some(idx) = state.queue_selected() {
                handle.cmd_tx.send(AudioCommand::PlayQueueIndex(idx))?;
            }
        }

        // === Log Tab ===

        fn log_up(KeyCode::Up, in_log) {
            log::trace!("Log scroll up");
        }

        fn log_down(KeyCode::Down, in_log) {
            log::trace!("Log scroll down");
        }

        fn settings_up(KeyCode::Up, in_settings) {
            if state.eq_state.show_eq {
                // In EQ panel: adjust gain up
                state.eq_state.local_master_gain = (state.eq_state.local_master_gain + 0.5).min(12.0);
                handle.cmd_tx.send(AudioCommand::EqSetMasterGain(state.eq_state.local_master_gain))?;
            } else {
                state.settings_state.prev_item();
            }
        }

        fn settings_down(KeyCode::Down, in_settings) {
            if state.eq_state.show_eq {
                // In EQ panel: adjust gain down
                state.eq_state.local_master_gain = (state.eq_state.local_master_gain - 0.5).max(-12.0);
                handle.cmd_tx.send(AudioCommand::EqSetMasterGain(state.eq_state.local_master_gain))?;
            } else {
                state.settings_state.next_item();
            }
        }

        fn settings_enter(KeyCode::Enter, in_settings) {
            if state.eq_state.show_eq {
                // In EQ panel: toggle EQ enabled
                state.eq_state.toggle_enabled();
                handle.cmd_tx.send(AudioCommand::EqSetEnabled(state.eq_state.eq_enabled))?;
            } else {
                // Navigate to EQ panel
                state.eq_state.open_panel();
            }
        }

        fn settings_esc(KeyCode::Esc, in_settings) {
            if state.eq_state.show_eq {
                // Close EQ panel
                state.eq_state.close_panel();
            }
        }

        fn settings_mode(KeyCode::Char('m'), in_settings) {
            if state.eq_state.show_eq {
                // Toggle Casual/Advanced mode
                state.eq_state.toggle_mode();
            }
        }

        fn settings_add_filter(KeyCode::Char('a'), in_settings) {
            if state.eq_state.show_eq {
                // Add a new filter band (max 8)
                if state.eq_state.local_filters.len() < 8 {
                    let new_id = state.eq_state.local_filters.len() as i16;
                    let new_filter = FilterNode::new(new_id, 1000.0); // Default 1kHz
                    state.eq_state.local_filters.push(new_filter);
                    // Sync all filters to engine
                    handle.cmd_tx.send(AudioCommand::EqSetAllFilters(state.eq_state.local_filters.clone()))?;
                    log::info!("Added new filter band at 1000 Hz");
                }
            }
        }
    })
}
