pub mod app;
pub mod event;
pub mod theme;
pub mod ui;

use std::io;
use std::time::Duration;

use crossterm::event::Event;
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::execute;
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use prexp_core::source::ProcessSource;

use app::App;

/// Run the TUI event loop.
pub fn run_tui(source: &dyn ProcessSource, refresh_interval: Duration) -> anyhow::Result<()> {
    // Setup terminal.
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(refresh_interval);

    // Initial data load.
    app.refresh(source);

    let result = run_loop(&mut terminal, &mut app, source);

    // Restore terminal.
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

fn run_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    source: &dyn ProcessSource,
) -> anyhow::Result<()> {
    loop {
        terminal.draw(|f| ui::draw(f, app))?;

        // Wait for at least one event, then drain all queued events
        // before redrawing. This prevents backlog when keys are held down.
        let poll_timeout = Duration::from_millis(100);
        if let Some(evt) = event::poll_event(poll_timeout) {
            if let Event::Key(key) = evt {
                event::handle_key(app, key, source);
            }
            // Drain remaining queued events without waiting.
            while let Some(evt) = event::poll_event(Duration::ZERO) {
                if let Event::Key(key) = evt {
                    event::handle_key(app, key, source);
                }
                if app.should_quit {
                    break;
                }
            }
        }

        if app.should_quit {
            return Ok(());
        }

        // Auto-refresh.
        if app.needs_refresh() {
            app.refresh(source);
        }
    }
}
