use std::path::{Path, PathBuf};
use crate::pbs::job_number;
use std::fs;
use std::io::{Read, Seek, SeekFrom};
use crate::app::Update;
use notify::{Event, RecursiveMode, Watcher};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::Duration;

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
        // File was truncated/rotated: re-read its whole tail.
        return read_tail(path, 64 * 1024);
    }
    f.seek(SeekFrom::Start(offset))?;
    let mut buf = String::new();
    f.read_to_string(&mut buf)?;
    Ok((buf, len))
}

const TAIL_BYTES: u64 = 64 * 1024;

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
                                            lines: text.lines().map(str::to_string).collect(),
                                            replace: true,
                                        });
                                    }
                                    Err(_) => {
                                        let _ = updates.send(Update::Log {
                                            lines: vec!["Log file not found or not accessible.".into()],
                                            replace: true,
                                        });
                                    }
                                }
                            }
                            None => {
                                let _ = updates.send(Update::Log { lines: Vec::new(), replace: true });
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
                        if let Ok((text, new_off)) = read_since(&p, offset) {
                            offset = new_off;
                            if !text.is_empty() {
                                let _ = updates.send(Update::Log {
                                    lines: text.lines().map(str::to_string).collect(),
                                    replace: false,
                                });
                            }
                        }
                    }
                }
                Ok(Err(_)) => {}
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
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
