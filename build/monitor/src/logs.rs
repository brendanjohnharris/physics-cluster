use std::path::{Path, PathBuf};
use crate::pbs::job_number;
use std::fs;
use std::io::{Read, Seek, SeekFrom};
use crate::app::Update;
use notify::{Event, RecursiveMode, Watcher};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::Duration;

/// Make a raw log line safe to render in a ratatui `Paragraph`.
///
/// ratatui measures each character with `unicode-width` and prints the byte
/// verbatim, but it does NOT emulate control characters: a `\t` is width 0 to
/// ratatui yet jumps the real cursor to the next tab stop, and a stray ESC/`\r`
/// makes the terminal move or repaint on its own. Either way ratatui's cell
/// buffer desyncs from the terminal and the frame diff can never repair it ---
/// the garbled, "sticky" log text. We therefore expand tabs to 8-column stops
/// and drop every other control character (whole ANSI escape sequences included)
/// at ingestion, so what reaches the renderer advances the cursor exactly as
/// ratatui expects.
pub fn sanitize_log_line(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut col = 0usize;
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\t' => {
                let spaces = 8 - (col % 8);
                out.extend(std::iter::repeat(' ').take(spaces));
                col += spaces;
            }
            '\u{1b}' => {
                // Drop an ANSI escape sequence. CSI ("ESC [") runs until a final
                // byte in 0x40..=0x7e; a lone ESC just gets dropped.
                if chars.peek() == Some(&'[') {
                    chars.next();
                    while let Some(&n) = chars.peek() {
                        chars.next();
                        if ('\u{40}'..='\u{7e}').contains(&n) {
                            break;
                        }
                    }
                }
            }
            // Drop other control characters (CR, BEL, BS, vertical tab, ...).
            c if c.is_control() => {}
            c => {
                out.push(c);
                // Tab-stop accounting; one column per char is close enough for the
                // ASCII-dominated logs we render (only affects tab alignment, never
                // correctness, since no control bytes survive to desync the cursor).
                col += 1;
            }
        }
    }
    out
}

/// Resolve the log file for a job id, mirroring the old bash logic.
pub fn resolve_log_path(jobs_dir: &Path, job_id: &str) -> Option<PathBuf> {
    // Array task: "1234[5].host" -> dir "1234[]*", file "<dir>/5.log".
    if let (Some(lb), Some(rb)) = (job_id.find('['), job_id.find(']')) {
        if rb > lb + 1 {
            let task_id = &job_id[lb + 1..rb];
            let num = job_number(job_id);
            let prefix = format!("{}[]", num);
            if let Ok(entries) = fs::read_dir(jobs_dir) {
                for e in entries.flatten() {
                    if !e.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                        continue;
                    }
                    let name = e.file_name();
                    let name = name.to_string_lossy();
                    if name.starts_with(&prefix) {
                        let candidate = e.path().join(format!("{}.log", task_id));
                        if candidate.is_file() {
                            return Some(candidate);
                        }
                    }
                }
            }
        }
    }

    // Regular job: glob "<jobnum>*.log".
    let num = job_number(job_id);
    if let Ok(entries) = fs::read_dir(jobs_dir) {
        for e in entries.flatten() {
            let name = e.file_name();
            let name = name.to_string_lossy();
            if name.starts_with(num) && name.ends_with(".log") {
                let p = e.path();
                if p.is_file() {
                    return Some(p);
                }
            }
        }
    }

    // Fallback: exact "<full-id>.log".
    let fallback = jobs_dir.join(format!("{}.log", job_id));
    if fallback.is_file() {
        Some(fallback)
    } else {
        None
    }
}

/// Read up to `max_bytes` from the end of the file. If the file is larger, the
/// first (partial) line is dropped so output starts on a line boundary.
/// Returns (text, file_len) where file_len is the byte offset read up to.
pub fn read_tail(path: &Path, max_bytes: u64) -> std::io::Result<(String, u64)> {
    let mut f = fs::File::open(path)?;
    let len = f.metadata()?.len();
    let start = len.saturating_sub(max_bytes);
    f.seek(SeekFrom::Start(start))?;
    let mut buf = String::new();
    f.read_to_string(&mut buf)?;
    if start > 0 {
        if let Some(nl) = buf.find('\n') {
            buf = buf[nl + 1..].to_string();
        }
    }
    Ok((buf, len))
}

/// Read bytes appended after `offset`. Returns (text, new_len).
pub fn read_since(path: &Path, offset: u64) -> std::io::Result<(String, u64)> {
    let mut f = fs::File::open(path)?;
    let len = f.metadata()?.len();
    if len < offset {
        // File was truncated/rotated: re-read from the start.
        return read_tail(path, TAIL_BYTES);
    }
    f.seek(SeekFrom::Start(offset))?;
    let mut buf = String::new();
    f.read_to_string(&mut buf)?;
    Ok((buf, len))
}

// Initial read budget: large enough to load the whole log for any real job, so the
// entire file is navigable (Home/End reach its true top/bottom); only a pathologically
// huge log is tailed to the last 64 MiB.
const TAIL_BYTES: u64 = 64 * 1024 * 1024;

/// Shown in the log preview when neither the `~/.jobs` log nor the PBS `Output_Path`
/// file can be found (e.g. a job that writes its output nowhere tail-able).
fn no_log_message() -> Vec<String> {
    vec![
        "No log file found for this job.".into(),
        String::new(),
        "To see live output, tee it to ~/.jobs/$PBS_JOBID.log in your".into(),
        "job script, or submit with '#PBS -k oed' (writes -o live).".into(),
    ]
}

fn pump_log_delta(path: &Path, offset: &mut u64, updates: &Sender<Update>) {
    let old_off = *offset;
    let len = match fs::metadata(path) {
        Ok(m) => m.len(),
        Err(_) => return,
    };

    // Fast path: no visible growth/truncation; avoid reopening the file.
    if len == old_off {
        return;
    }

    if let Ok((text, new_off)) = read_since(path, old_off) {
        *offset = new_off;

        // On truncate/rotation, replace the preview so old lines do not linger.
        let replace = len < old_off;
        if !text.is_empty() || replace {
            let _ = updates.send(Update::Log {
                lines: text.lines().map(sanitize_log_line).collect(),
                replace,
            });
        }
    }
}

pub fn spawn_log_watcher(updates: Sender<Update>) -> Sender<Option<PathBuf>> {
    let (path_tx, path_rx): (Sender<Option<PathBuf>>, Receiver<Option<PathBuf>>) = channel();

    std::thread::spawn(move || {
        // notify delivers filesystem events into ev_rx.
        let (ev_tx, ev_rx) = channel::<notify::Result<Event>>();
        let mut watcher = match notify::recommended_watcher(move |res| {
            let _ = ev_tx.send(res);
        }) {
            Ok(w) => w,
            Err(e) => {
                let _ = updates.send(Update::Error {
                    source: "log".into(),
                    message: format!("watcher init failed: {e}"),
                });
                return;
            }
        };

        let mut current: Option<PathBuf> = None;
        let mut offset: u64 = 0;

        loop {
            // 1. Drain any pending path switches.
            loop {
                match path_rx.try_recv() {
                    Ok(new_path) => {
                        if let Some(old) = &current {
                            let _ = watcher.unwatch(old);
                        }
                        current = new_path.clone();
                        offset = 0;
                        match new_path {
                            Some(p) => {
                                match read_tail(&p, TAIL_BYTES) {
                                    Ok((text, off)) => {
                                        offset = off;
                                        let _ = watcher.watch(&p, RecursiveMode::NonRecursive);
                                        let _ = updates.send(Update::Log {
                                            lines: text.lines().map(sanitize_log_line).collect(),
                                            replace: true,
                                        });
                                    }
                                    Err(_) => {
                                        let _ = updates.send(Update::Log {
                                            lines: no_log_message(),
                                            replace: true,
                                        });
                                    }
                                }
                            }
                            None => {
                                let _ = updates.send(Update::Log {
                                    lines: no_log_message(),
                                    replace: true,
                                });
                            }
                        }
                    }
                    Err(std::sync::mpsc::TryRecvError::Empty) => break,
                    Err(std::sync::mpsc::TryRecvError::Disconnected) => return,
                }
            }

            // 2. Wait for a filesystem event (or wake every 250ms to recheck paths).
            match ev_rx.recv_timeout(Duration::from_millis(250)) {
                Ok(Ok(_event)) => {
                    if let Some(p) = current.clone() {
                        pump_log_delta(&p, &mut offset, &updates);
                    }
                }
                Ok(Err(_)) => {}
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                    // Fallback path: if notify misses an event (or loses a watch),
                    // still advance from the file length delta.
                    if let Some(p) = current.clone() {
                        pump_log_delta(&p, &mut offset, &updates);
                    }
                }
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
            }
        }
    });

    path_tx
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn tmpdir(tag: &str) -> PathBuf {
        let mut d = std::env::temp_dir();
        d.push(format!("monitor_logtest_{}_{}", tag, std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn resolves_regular_job_by_glob() {
        let dir = tmpdir("reg");
        fs::write(dir.join("14156.headnode.log"), "hello").unwrap();
        let p = resolve_log_path(&dir, "14156.headnode").unwrap();
        assert_eq!(p, dir.join("14156.headnode.log"));
    }

    #[test]
    fn resolves_array_task_in_bracket_dir() {
        let dir = tmpdir("arr");
        let jobdir = dir.join("16050[].headnode");
        fs::create_dir_all(&jobdir).unwrap();
        fs::write(jobdir.join("3.log"), "task3").unwrap();
        let p = resolve_log_path(&dir, "16050[3].headnode").unwrap();
        assert_eq!(p, jobdir.join("3.log"));
    }

    #[test]
    fn returns_none_when_absent() {
        let dir = tmpdir("none");
        assert_eq!(resolve_log_path(&dir, "99999.headnode"), None);
    }

    #[test]
    fn sanitize_expands_tabs_to_eight_col_stops() {
        // "From worker 3:" is 14 chars; the tab fills to column 16.
        let line = "From worker 3:\t\u{250c} Info: hi";
        let out = sanitize_log_line(line);
        assert_eq!(out, "From worker 3:  \u{250c} Info: hi");
        // No tab or other control byte survives.
        assert!(!out.contains('\t'));
        // Box-drawing characters are not controls and must be preserved.
        assert!(out.contains('\u{250c}'));
    }

    #[test]
    fn sanitize_strips_ansi_and_control_bytes() {
        let line = "\u{1b}[31mERROR\u{1b}[0m boom\r\u{7}";
        let out = sanitize_log_line(line);
        assert_eq!(out, "ERROR boom");
        assert!(!out.contains('\u{1b}'));
        assert!(!out.contains('\r'));
    }

    #[test]
    fn sanitize_tab_stop_resets_each_call() {
        // A leading tab from column 0 expands to a full 8 spaces.
        assert_eq!(sanitize_log_line("\tx"), "        x");
        // Plain text is untouched.
        assert_eq!(sanitize_log_line("plain line"), "plain line");
    }

    #[test]
    fn reads_tail_within_byte_budget() {
        let dir = tmpdir("tail");
        let f = dir.join("a.log");
        let body: String = (0..100).map(|i| format!("line{}\n", i)).collect();
        fs::write(&f, &body).unwrap();
        let (text, off) = read_tail(&f, 40).unwrap();
        assert!(text.ends_with("line99\n"));
        assert!(text.len() <= 40 + 16); // tail of file, partial first line trimmed
        assert_eq!(off, body.len() as u64);
    }
}
