use std::io;
use std::time::Duration;

use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::backend::CrosstermBackend;

use audido_core::{
    commands::AudioCommand,
    engine::{AudioEngine, AudioEngineHandle},
};

mod state;
mod ui;

use state::{AppState, FocusedWidget};

fn main() -> anyhow::Result<()> {
    // Initialize tui_logger for TUI log display
    tui_logger::init_logger(log::LevelFilter::Debug).expect("Failed to init tui_logger");
    tui_logger::set_default_level(log::LevelFilter::Debug);

    log::info!("Starting Audido TUI");

    // Get audio file path from command line args
    let args: Vec<String> = std::env::args().collect();
    let audio_file = args.get(1).cloned();

    // Create audio engine and get communication handle
    let (engine, handle) = AudioEngine::new()?;

    // Spawn audio engine on dedicated thread
    let _engine_thread = engine.spawn();

    // Run TUI
    let result = run_tui(handle, audio_file);

    // Ensure clean shutdown
    result
}

fn run_tui(handle: AudioEngineHandle, initial_file: Option<String>) -> anyhow::Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = ratatui::Terminal::new(backend)?;

    let mut state = AppState::new();

    // Load initial file if provided
    if let Some(path) = initial_file {
        handle.cmd_tx.send(AudioCommand::Load(path.clone()))?;
        state.status_message = format!("Loading: {}", path);
        log::info!("Loading audio file: {}", path);
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

                        // Focus switching
                        KeyCode::Tab | KeyCode::Esc => {
                            state.toggle_focus();
                            log::debug!("Switched focus to {:?}", state.focused_widget);
                        }

                        // Context-sensitive arrow keys
                        KeyCode::Up => {
                            match state.focused_widget {
                                FocusedWidget::Playback => {
                                    state.volume = (state.volume + 0.1).min(1.0);
                                    handle.cmd_tx.send(AudioCommand::SetVolume(state.volume))?;
                                }
                                FocusedWidget::Log => {
                                    // tui_logger handles scrolling internally via its state
                                    // For basic usage, we just log that scrolling is attempted
                                    log::trace!("Log scroll up");
                                }
                            }
                        }
                        KeyCode::Down => match state.focused_widget {
                            FocusedWidget::Playback => {
                                state.volume = (state.volume - 0.1).max(0.0);
                                handle.cmd_tx.send(AudioCommand::SetVolume(state.volume))?;
                            }
                            FocusedWidget::Log => {
                                log::trace!("Log scroll down");
                            }
                        },
                        KeyCode::Left => {
                            if state.focused_widget == FocusedWidget::Playback {
                                let new_pos = (state.position - 5.0).max(0.0);
                                handle.cmd_tx.send(AudioCommand::Seek(new_pos))?;
                            }
                        }
                        KeyCode::Right => {
                            if state.focused_widget == FocusedWidget::Playback {
                                let new_pos = state.position + 5.0;
                                handle.cmd_tx.send(AudioCommand::Seek(new_pos))?;
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
