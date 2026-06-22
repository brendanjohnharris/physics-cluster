use monitor::app::{App, Update};
use monitor::poller::{self, PollIntervals};
use monitor::{details, logs, ssh, ui};

use clap::Parser;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io::stdout;
use std::path::PathBuf;
use std::sync::mpsc::channel;
use std::time::Duration;

#[derive(Parser)]
#[command(name = "monitor", about = "PBS job + cluster monitor (gentle polling)")]
struct Cli {
    /// User whose jobs to monitor (default: $USER)
    username: Option<String>,
    /// Cluster-load poll interval, seconds
    #[arg(long, default_value_t = 30)]
    cluster_interval: u64,
    /// Your-jobs poll interval, seconds
    #[arg(long, default_value_t = 10)]
    jobs_interval: u64,
}

/// Restores the terminal on drop, even on panic.
struct TermGuard;
impl Drop for TermGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(stdout(), LeaveAlternateScreen);
    }
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let user = cli
        .username
        .or_else(|| std::env::var("USER").ok())
        .unwrap_or_default();

    // Best-effort: make qlload's internal SSH cheap. Never fatal.
    let _ = ssh::ensure_controlmaster();

    let jobs_dir = {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
        PathBuf::from(home).join(".jobs")
    };

    // Channels: poller + log watcher both send Updates to the main loop.
    let (update_tx, update_rx) = channel::<Update>();
    let force_tx = poller::spawn_poller(
        user,
        PollIntervals {
            jobs: Duration::from_secs(cli.jobs_interval),
            cluster: Duration::from_secs(cli.cluster_interval),
        },
        update_tx.clone(),
    );
    let log_path_tx = logs::spawn_log_watcher(update_tx.clone());
    let detail_tx = details::spawn_details_fetcher(update_tx.clone());

    // Install panic hook before entering raw mode so a panic at any point restores the terminal.
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = disable_raw_mode();
        let _ = execute!(stdout(), LeaveAlternateScreen);
        default_hook(info);
    }));
    // Terminal setup with a guard so we always restore on exit/panic.
    enable_raw_mode()?;
    execute!(stdout(), EnterAlternateScreen)?;
    let _guard = TermGuard;

    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;
    let mut app = App::new();
    let mut current_log_job: Option<String> = None;

    loop {
        // Drain all pending updates from background threads.
        while let Ok(u) = update_rx.try_recv() {
            app.apply(u);
        }

        // If the selected job changed, point the log watcher at its file.
        let sel_id = app.selected_job_id.clone();
        if sel_id != current_log_job {
            current_log_job = sel_id.clone();
            let path = sel_id
                .as_ref()
                .and_then(|id| logs::resolve_log_path(&jobs_dir, id));
            let _ = log_path_tx.send(path);
            let _ = detail_tx.send(sel_id.clone());
        }

        terminal.draw(|f| ui::draw(f, &mut app))?;

        // Input with a short timeout so the clock/ages keep ticking.
        if event::poll(Duration::from_millis(150))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                match key.code {
                    // Quit (q now toggles the Queued section, so quit is Esc / Ctrl-C)
                    KeyCode::Esc => app.should_quit = true,
                    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        app.should_quit = true
                    }
                    // Switch the top tab (System Load <-> Log Preview)
                    KeyCode::Left => app.prev_tab(),
                    KeyCode::Right => app.next_tab(),
                    // Scroll the active top tab
                    KeyCode::Up | KeyCode::Char('k') => app.scroll_up(),
                    KeyCode::Down | KeyCode::Char('j') => app.scroll_down(),
                    KeyCode::Char('g') | KeyCode::Home => app.scroll_to_top(),
                    KeyCode::Char('G') | KeyCode::End => app.scroll_to_bottom(),
                    // Switch the selected running job (drives the Log Preview tab)
                    KeyCode::PageDown => app.page_down(),
                    KeyCode::PageUp => app.page_up(),
                    KeyCode::Char('.') => app.next_job(),
                    KeyCode::Char(',') => app.prev_job(),
                    // Toggle the lower sections
                    KeyCode::Char('a') => app.toggle_array(),
                    KeyCode::Char('q') => app.toggle_queued(),
                    KeyCode::Char('u') => app.toggle_running(),
                    // Force an immediate PBS refresh
                    KeyCode::Char('r') => {
                        let _ = force_tx.send(());
                        let _ = detail_tx.send(app.selected_job_id.clone());
                    }
                    _ => {}
                }
            }
        }

        if app.should_quit {
            break;
        }
    }

    Ok(())
}
