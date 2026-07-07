use monitor::app::{App, Update};
use monitor::poller::{self, PollCmd, PollIntervals};
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
    /// User to monitor and highlight (default: $USER); switch live with `u`
    #[arg(short = 'u', long)]
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
    let poll_tx = poller::spawn_poller(
        user.clone(),
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
    app.user = user;
    let mut current_log_job: Option<String> = None;
    let mut log_resolved = false; // false => selected job's log file not found yet, keep retrying

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
            log_resolved = path.is_some() || sel_id.is_none();
            let _ = log_path_tx.send(path);
            let _ = detail_tx.send(sel_id.clone());
        } else if !log_resolved {
            // Job was selected before its log file existed; re-resolve each tick
            // (the loop already wakes ~every 150ms) until it appears, then point
            // the watcher at it. Path only --- don't re-fetch details.
            if let Some(id) = &sel_id {
                if let Some(path) = logs::resolve_log_path(&jobs_dir, id) {
                    log_resolved = true;
                    let _ = log_path_tx.send(Some(path));
                } else if app.details_job.as_deref() == Some(id.as_str()) {
                    // Fallback: no ~/.jobs log, but the PBS Output_Path file exists
                    // (parsed from qstat -f by the details worker). Uses no extra call.
                    if let Some(p) = app.output_path.clone().filter(|p| p.is_file()) {
                        log_resolved = true;
                        let _ = log_path_tx.send(Some(p));
                    }
                }
            }
        }

        terminal.draw(|f| ui::draw(f, &mut app))?;

        // Input with a short timeout so the clock/ages keep ticking.
        if event::poll(Duration::from_millis(150))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
                // Ctrl-C always quits, even while typing in the username box.
                if ctrl && key.code == KeyCode::Char('c') {
                    app.should_quit = true;
                } else if app.input.is_some() {
                    // Username box open: keys edit the buffer instead of driving the UI.
                    match key.code {
                        KeyCode::Enter => {
                            if let Some(new_user) = app.confirm_input() {
                                let _ = poll_tx.send(PollCmd::SetUser(new_user));
                            }
                        }
                        KeyCode::Esc => app.cancel_input(),
                        KeyCode::Backspace => app.input_backspace(),
                        KeyCode::Char(c) => app.input_push(c),
                        _ => {}
                    }
                } else {
                    match key.code {
                        // Quit (q toggles the Queued section, so quit is Esc / Ctrl-C)
                        KeyCode::Esc => app.should_quit = true,
                        // Switch the top tab (System Load <-> Log Preview <-> Details)
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
                        // Toggle the lower sections (running moved off 'u' to 'r')
                        KeyCode::Char('a') => app.toggle_array(),
                        KeyCode::Char('q') => app.toggle_queued(),
                        KeyCode::Char('r') if !ctrl => app.toggle_running(),
                        // Toggle array-job compaction (e/c are redundant mnemonics)
                        KeyCode::Char('e') | KeyCode::Char('c') => app.toggle_compact(),
                        // Open the username switch box
                        KeyCode::Char('u') => app.begin_user_input(),
                        // Force an immediate PBS refresh (moved off 'r' to Ctrl-R)
                        KeyCode::Char('r') if ctrl => {
                            let _ = poll_tx.send(PollCmd::Refresh);
                            let _ = detail_tx.send(app.selected_job_id.clone());
                        }
                        _ => {}
                    }
                }
            }
        }

        if app.should_quit {
            break;
        }
    }

    Ok(())
}
