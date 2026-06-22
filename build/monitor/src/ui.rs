use crate::app::{App, TopTab};
use crate::pbs::JobState;
use ansi_to_tui::IntoText;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

const LEGEND: &str =
    " ←/→ tab · ↑/↓ PgUp/PgDn Home/End scroll · ,/. job · a/q/u sections · r refresh · Esc quit";

/// Parse "HH:MM" or "HH:MM:SS" to total minutes (seconds ignored).
fn hhmm_to_mins(s: &str) -> Option<u64> {
    let mut parts = s.split(':');
    let h: u64 = parts.next()?.trim().parse().ok()?;
    let m: u64 = parts.next()?.trim().parse().ok()?;
    Some(h * 60 + m)
}

/// A fixed-width walltime progress bar `[####----] NN%` from elapsed/req ("HH:MM").
fn walltime_bar(elapsed: &str, req: &str, width: usize) -> String {
    match (hhmm_to_mins(elapsed), hhmm_to_mins(req)) {
        (Some(e), Some(r)) if r > 0 => {
            let frac = (e as f64 / r as f64).clamp(0.0, 1.0);
            let filled = ((frac * width as f64).round() as usize).min(width);
            let pct = (frac * 100.0).round() as u32;
            format!(
                "[{}{}] {:>3}%",
                "#".repeat(filled),
                "-".repeat(width - filled),
                pct
            )
        }
        _ => format!("[{}] ---", "-".repeat(width)),
    }
}

/// Lines for jobs in a class: want_running=true => Running; false => Queued|Held.
/// Running rows show ncpus, req mem, and an elapsed/req walltime bar; queued/held
/// rows show ncpus, mem, state, and requested walltime.
fn job_lines(app: &App, want_running: bool) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    for job in app.jobs.iter() {
        let is_running = job.state == JobState::Running;
        let in_class = if want_running {
            is_running
        } else {
            matches!(job.state, JobState::Queued | JobState::Held)
        };
        if !in_class {
            continue;
        }
        let selected = app.selected_job().map(|j| j.id == job.id).unwrap_or(false);
        let cpus = job
            .cpus
            .map(|c| format!("{c}c"))
            .unwrap_or_else(|| "--".into());
        let mem = job.mem.clone().unwrap_or_else(|| "--".into());
        let node = job.node.clone().unwrap_or_else(|| "--".into());
        let raw = if want_running {
            let bar = match (&job.elapsed, &job.req_walltime) {
                (Some(e), Some(r)) => walltime_bar(e, r, 8),
                _ => format!("[{}] ---", "-".repeat(8)),
            };
            format!(
                "{:<15} {:<10} {:>4} {:>7}  {}  {}",
                job.id, job.name, cpus, mem, bar, node
            )
        } else {
            let state = match &job.state {
                JobState::Queued => "Q",
                JobState::Held => "H",
                _ => "?",
            };
            let req = job.req_walltime.clone().unwrap_or_else(|| "--".into());
            format!(
                "{:<15} {:<10} {:>4} {:>7}  {}  req {}",
                job.id, job.name, cpus, mem, state, req
            )
        };
        let style = if selected && want_running {
            Style::default()
                .fg(Color::Red)
                .add_modifier(Modifier::REVERSED | Modifier::BOLD)
        } else {
            Style::default()
        };
        lines.push(Line::from(Span::styled(raw, style)));
    }
    lines
}

fn freshness_line(app: &App) -> Line<'static> {
    let job = match app.selected_job() {
        Some(j) => format!("job {}/{} {}", app.selected_ordinal(), app.running_count(), j.id),
        None => "no running jobs".to_string(),
    };
    let load = app
        .cluster_at
        .map(|t| format!("load {}s", t.elapsed().as_secs()))
        .unwrap_or_else(|| "load --".into());
    let jobs = app
        .jobs_at
        .map(|t| format!("jobs {}s", t.elapsed().as_secs()))
        .unwrap_or_else(|| "jobs --".into());
    let mut s = format!("{job} · {load} · {jobs} ");
    if let Some((msg, _)) = &app.last_error {
        s = format!("!{msg} · {s}");
    }
    Line::from(Span::styled(s, Style::default().fg(Color::DarkGray)))
}

fn tab_title(app: &App) -> Line<'static> {
    // Each tab keeps its own fixed colour; only the active tab gains the
    // reverse-video/bold highlight.
    let tab = |label: &'static str, color: Color, active: bool| {
        let mut style = Style::default().fg(color);
        if active {
            style = style.add_modifier(Modifier::REVERSED | Modifier::BOLD);
        }
        Span::styled(label, style)
    };
    Line::from(vec![
        tab(
            " System Load ",
            Color::Blue,
            app.top_tab == TopTab::SystemLoad,
        ),
        Span::raw(" "),
        tab(
            " Log Preview ",
            Color::Yellow,
            app.top_tab == TopTab::LogPreview,
        ),
        Span::raw(" "),
        tab(" Details ", Color::Cyan, app.top_tab == TopTab::Details),
    ])
}

fn render_top(f: &mut Frame, area: Rect, app: &mut App) {
    let block = Block::default().borders(Borders::TOP).title(tab_title(app));
    let inner_h = area.height.saturating_sub(1) as usize; // minus the TOP border row
    let tab = app.top_tab;
    app.top_inner_h = inner_h;

    let text: Text = match tab {
        TopTab::SystemLoad => app
            .cluster
            .as_str()
            .into_text()
            .unwrap_or_else(|_| Text::raw(app.cluster.clone())),
        TopTab::LogPreview => {
            Text::from(app.log.iter().map(|l| Line::from(l.clone())).collect::<Vec<_>>())
        }
        TopTab::Details => Text::raw(app.details.clone()),
    };
    let total = text.lines.len();
    let max_off = total.saturating_sub(inner_h);

    // Borrow the active tab's follow flag + scroll offset (disjoint fields).
    let (follow, scroll): (&mut bool, &mut usize) = match tab {
        TopTab::SystemLoad => (&mut app.cluster_follow, &mut app.cluster_scroll),
        TopTab::LogPreview => (&mut app.log_follow, &mut app.log_scroll),
        TopTab::Details => (&mut app.details_follow, &mut app.details_scroll),
    };
    let off = if *follow {
        *scroll = max_off; // pin to the bottom
        max_off
    } else {
        let clamped = (*scroll).min(max_off);
        *scroll = clamped; // write back the clamp (prevents unbounded offsets)
        if clamped >= max_off {
            *follow = true; // scrolled back to the bottom -> re-engage follow
        }
        clamped
    };
    let off = off.min(u16::MAX as usize) as u16;
    f.render_widget(Paragraph::new(text).scroll((off, 0)).block(block), area);
}

fn render_section(f: &mut Frame, area: Rect, title: &str, color: Color, content: Text<'static>) {
    let block = Block::default().borders(Borders::TOP).title(Span::styled(
        format!(" {title} "),
        Style::default().fg(color).add_modifier(Modifier::BOLD),
    ));
    f.render_widget(Paragraph::new(content).block(block), area);
}

#[derive(Clone, Copy)]
enum Slot {
    Top,
    Gap,
    Running,
    Queued,
    Array,
    Status,
}

pub fn draw(f: &mut Frame, app: &mut App) {
    let area = f.area();

    let running = job_lines(app, true);
    let queued = job_lines(app, false);
    let array_text: Option<Text> = if app
        .array
        .as_ref()
        .map(|s| !s.trim().is_empty())
        .unwrap_or(false)
    {
        let raw = app.array.clone().unwrap();
        Some(raw.as_str().into_text().unwrap_or_else(|_| Text::raw(raw)))
    } else {
        None
    };

    let show_running = !running.is_empty() && app.show_running;
    let show_queued = !queued.is_empty() && app.show_queued;
    let show_array = array_text.is_some() && app.show_array;

    let mut slots: Vec<(Slot, Constraint)> = vec![(Slot::Top, Constraint::Min(3))];
    if show_running {
        slots.push((Slot::Gap, Constraint::Length(1)));
        slots.push((Slot::Running, Constraint::Length(1 + running.len() as u16)));
    }
    if show_queued {
        slots.push((Slot::Gap, Constraint::Length(1)));
        slots.push((Slot::Queued, Constraint::Length(1 + queued.len() as u16)));
    }
    if show_array {
        let n = array_text.as_ref().unwrap().lines.len() as u16;
        slots.push((Slot::Gap, Constraint::Length(1)));
        slots.push((Slot::Array, Constraint::Length(1 + n)));
    }
    slots.push((Slot::Status, Constraint::Length(1)));

    let constraints: Vec<Constraint> = slots.iter().map(|(_, c)| *c).collect();
    let rects = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(area);

    for ((slot, _), rect) in slots.iter().zip(rects.iter()) {
        match slot {
            Slot::Top => render_top(f, *rect, app),
            Slot::Gap => {}
            Slot::Running => {
                render_section(f, *rect, "RUNNING JOBS", Color::Green, Text::from(running.clone()))
            }
            Slot::Queued => render_section(
                f,
                *rect,
                "QUEUED & HELD JOBS",
                Color::Yellow,
                Text::from(queued.clone()),
            ),
            Slot::Array => render_section(
                f,
                *rect,
                "ARRAY JOB PROGRESS",
                Color::Magenta,
                array_text.clone().unwrap(),
            ),
            Slot::Status => {
                let fresh_w = rect.width.saturating_sub(40).min(48);
                let chunks = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([Constraint::Min(0), Constraint::Length(fresh_w)])
                    .split(*rect);
                f.render_widget(
                    Paragraph::new(Line::from(Span::styled(
                        LEGEND,
                        Style::default().fg(Color::Cyan),
                    ))),
                    chunks[0],
                );
                f.render_widget(
                    Paragraph::new(freshness_line(app)).alignment(Alignment::Right),
                    chunks[1],
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::Update;
    use crate::pbs::Job;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    fn job(id: &str, state: JobState, node: Option<&str>) -> Job {
        Job {
            id: id.into(),
            owner: "bhar9988".into(),
            queue: "l40s".into(),
            name: "code".into(),
            state,
            node: node.map(|s| s.into()),
            cpus: None,
            mem: None,
            req_walltime: None,
            elapsed: None,
        }
    }

    #[test]
    fn renders_tabs_sections_and_status() {
        let mut app = App::new();
        app.apply(Update::Cluster("node load line".into()));
        app.apply(Update::Jobs(vec![
            Job {
                id: "15992.headnode".into(),
                owner: "bhar9988".into(),
                queue: "l40s".into(),
                name: "code".into(),
                state: JobState::Running,
                node: Some("nodegpu02".into()),
                cpus: Some(16),
                mem: Some("88gb".into()),
                req_walltime: Some("23:00".into()),
                elapsed: Some("03:34".into()),
            },
            job("16001.headnode", JobState::Queued, None),
        ]));
        app.apply(Update::Log {
            lines: vec!["log line one".into()],
            replace: true,
        });

        let backend = TestBackend::new(80, 30);
        let mut term = Terminal::new(backend).unwrap();
        term.draw(|f| draw(f, &mut app)).unwrap();
        let dump: String = term.backend().buffer().content().iter().map(|c| c.symbol()).collect();
        assert!(dump.contains("System Load"));
        assert!(dump.contains("Log Preview"));
        assert!(dump.contains("RUNNING JOBS"));
        assert!(dump.contains("QUEUED & HELD JOBS"));
        assert!(dump.contains("15992.headnode"));
        assert!(dump.contains("16001.headnode"));
        assert!(dump.contains("scroll")); // status-bar legend
        assert!(dump.contains("node load line"));
        assert!(dump.contains("16c")); // ncpus column
        assert!(dump.contains("16%")); // walltime bar pct (elapsed 03:34 / req 23:00)

        // System Load is the default tab, so the log is not shown until we switch.
        assert!(!dump.contains("log line one"));
        app.next_tab();
        term.draw(|f| draw(f, &mut app)).unwrap();
        let dump2: String = term.backend().buffer().content().iter().map(|c| c.symbol()).collect();
        assert!(dump2.contains("log line one"));
    }
}
