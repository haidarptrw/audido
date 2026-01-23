use std::fs::canonicalize;
use std::{io, path::PathBuf};
use std::time::Duration;

use audido_core::browser;
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

mod state;
mod ui;

use state::{ActiveTab, AppState, BrowserFileDialog};

fn main() -> anyhow::Result<()> {
    // Initialize tui_logger for TUI log display
    tui_logger::init_logger(log::LevelFilter::Debug).expect("Failed to init tui_logger");
    tui_logger::set_default_level(log::LevelFilter::Debug);

    log::info!("Starting Audido TUI");

    // Get audio file paths from command line args (supports multiple files)
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

    // Load initial files if provided (add to queue and start playing)
    if !initial_files.is_empty() {

        if let Some(first_part_str) = initial_files.first() {
            let path = PathBuf::from(first_part_str);

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

            if let Some(dir) = target_dir {{
                // Call core browser function to get file list
                if let Ok(items) = browser::get_directory_content(&dir) {
                    state.current_dir = dir;
                    state.browser_items = items;
                    state.browser_state.select(Some(0));
                    log::info!("Browser context set to: {:?}", state.current_dir);
                }
            }}
        }
        log::info!("Adding {} files to queue from CLI", initial_files.len());
        handle
            .cmd_tx
            .send(AudioCommand::AddToQueue(initial_files))?;
        handle.cmd_tx.send(AudioCommand::PlayQueueIndex(0))?;
        state.status_message = "Loading queue...".to_string();
    }

    loop {
        // Handle audio engine responses
        while let Ok(response) = handle.resp_rx.try_recv() {
            state.handle_response(response);
        }

        // Draw UI
        terminal.draw(|f| ui::draw(f, &state))?;

        // Handle input (with timeout to allow response polling)
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        // Global keys (work in both modes)
                        KeyCode::Char('q') => {
                            let _ = handle.cmd_tx.send(AudioCommand::Quit);
                            break;
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

                        // Tab switching (only when dialog is not open)
                        KeyCode::Tab => {
                            if !state.is_dialog_open() {
                                state.next_tab();
                                log::debug!("Switched to tab {:?}", state.active_tab);
                            }
                        }
                        KeyCode::Esc => {
                            if state.is_dialog_open() {
                                state.close_dialog();
                            } else {
                                state.next_tab();
                            }
                        }

                        // Context-sensitive arrow keys
                        KeyCode::Up => {
                            if state.is_dialog_open() {
                                state.dialog_toggle();
                            } else {
                                match state.active_tab {
                                    ActiveTab::Playback => {
                                        state.volume = (state.volume + 0.1).min(1.0);
                                        handle
                                            .cmd_tx
                                            .send(AudioCommand::SetVolume(state.volume))?;
                                    }
                                    ActiveTab::Log => {
                                        log::trace!("Log scroll up");
                                    }
                                    ActiveTab::Browser => state.browser_prev(),
                                    ActiveTab::Queue => state.queue_prev(),
                                }
                            }
                        }
                        KeyCode::Down => {
                            if state.is_dialog_open() {
                                state.dialog_toggle();
                            } else {
                                match state.active_tab {
                                    ActiveTab::Playback => {
                                        state.volume = (state.volume - 0.1).max(0.0);
                                        handle
                                            .cmd_tx
                                            .send(AudioCommand::SetVolume(state.volume))?;
                                    }
                                    ActiveTab::Log => {
                                        log::trace!("Log scroll down");
                                    }
                                    ActiveTab::Browser => state.browser_next(),
                                    ActiveTab::Queue => state.queue_next(),
                                }
                            }
                        }
                        KeyCode::Left => {
                            if state.active_tab == ActiveTab::Playback {
                                let new_pos = (state.position - 5.0).max(0.0);
                                handle.cmd_tx.send(AudioCommand::Seek(new_pos))?;
                            }
                        }
                        KeyCode::Right => {
                            if state.active_tab == ActiveTab::Playback {
                                let new_pos = state.position + 5.0;
                                handle.cmd_tx.send(AudioCommand::Seek(new_pos))?;
                            }
                        }
                        KeyCode::Enter => {
                            // Handle dialog confirmation
                            if let BrowserFileDialog::Open { path, selected } =
                                &state.browser_dialog
                            {
                                let path_str = path.to_string_lossy().to_string();
                                if *selected == 0 {
                                    // Play Now: clear queue, add this file, play
                                    handle.cmd_tx.send(AudioCommand::ClearQueue)?;
                                    handle
                                        .cmd_tx
                                        .send(AudioCommand::AddToQueue(vec![path_str]))?;
                                    handle.cmd_tx.send(AudioCommand::PlayQueueIndex(0))?;
                                    state.active_tab = ActiveTab::Playback;
                                } else {
                                    // Add to Queue
                                    handle
                                        .cmd_tx
                                        .send(AudioCommand::AddToQueue(vec![path_str]))?;
                                }
                                state.close_dialog();
                            } else if state.active_tab == ActiveTab::Browser {
                                // Open dialog for file selection
                                if let Some(path) = state.browser_enter() {
                                    state.open_browser_dialog(path);
                                }
                            } else if state.active_tab == ActiveTab::Queue {
                                // Play selected track from queue
                                if let Some(idx) = state.queue_selected() {
                                    handle.cmd_tx.send(AudioCommand::PlayQueueIndex(idx))?;
                                }
                            }
                        }
                        _ => {}
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
