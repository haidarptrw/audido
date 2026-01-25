use std::fs::canonicalize;
use std::time::Duration;
use std::{io, path::PathBuf};

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

mod router;
mod state;
mod ui;

use router::{PlaybackRoute, Router, route_for_name, tab_names};
use state::AppState;

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
    let mut router = Router::new(Box::new(PlaybackRoute));

    // Handle initial setup (Browser context & Queue loading)
    setup_initial_state(&mut state, &handle, initial_files)?;

    loop {
        // Handle audio engine responses
        while let Ok(response) = handle.resp_rx.try_recv() {
            state.handle_response(response);
        }

        // Draw UI
        terminal.draw(|f| ui::draw(f, &state, &router))?;

        // Handle input
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    // Handle global keys first
                    let should_quit =
                        handle_global_keys(key.code, &mut state, &handle, &mut router)?;
                    if should_quit {
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

/// Handle global keys and delegate route-specific input to router
fn handle_global_keys(
    key: KeyCode,
    state: &mut AppState,
    handle: &AudioEngineHandle,
    router: &mut Router,
) -> anyhow::Result<bool> {
    // Global keys that work regardless of route
    match key {
        KeyCode::Char('q') => {
            let _ = handle.cmd_tx.send(AudioCommand::Quit);
            return Ok(true);
        }
        KeyCode::Char(' ') => {
            if state.is_playing {
                handle.cmd_tx.send(AudioCommand::Pause)?;
            } else {
                handle.cmd_tx.send(AudioCommand::Play)?;
            }
            return Ok(false);
        }
        KeyCode::Char('s') => {
            handle.cmd_tx.send(AudioCommand::Stop)?;
            return Ok(false);
        }
        KeyCode::Char('n') => {
            handle.cmd_tx.send(AudioCommand::Next)?;
            return Ok(false);
        }
        KeyCode::Char('p') => {
            handle.cmd_tx.send(AudioCommand::Previous)?;
            return Ok(false);
        }
        KeyCode::Char('l') => {
            let next_mode = state.next_loop_mode();
            handle.cmd_tx.send(AudioCommand::SetLoopMode(next_mode))?;
            return Ok(false);
        }
        KeyCode::Tab => {
            // Cycle through tabs
            let tabs = tab_names();
            let current_name = router.current().name();
            let current_idx = tabs.iter().position(|n| *n == current_name).unwrap_or(0);
            let next_idx = (current_idx + 1) % tabs.len();
            let next_route = route_for_name(tabs[next_idx]);
            router.replace(next_route, state, handle)?;
            return Ok(false);
        }
        KeyCode::Esc => {
            // Try to pop from router (go back)
            if router.depth() > 1 {
                router.pop(state, handle)?;
                return Ok(false);
            }
        }
        _ => {}
    }

    // Delegate to the current route's input handler
    let action = router.current_mut().handle_input(key, state, handle)?;
    let should_quit = router.execute_action(action, state, handle)?;
    Ok(should_quit)
}
