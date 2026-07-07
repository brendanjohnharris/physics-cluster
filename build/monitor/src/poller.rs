use crate::app::Update;
use crate::pbs;
use std::sync::mpsc::{channel, Receiver, RecvTimeoutError, Sender};
use std::time::{Duration, Instant};

pub struct PollIntervals {
    pub jobs: Duration,
    pub cluster: Duration,
}

/// Commands the main thread sends the poller.
pub enum PollCmd {
    /// Force an immediate refresh of all PBS sources now.
    Refresh,
    /// Switch the monitored/highlighted user and refresh immediately.
    SetUser(String),
}

fn poll_jobs(user: &str, updates: &Sender<Update>) {
    match pbs::fetch_user_jobs(user) {
        Ok(jobs) => {
            let has_arrays = pbs::has_array_jobs(&jobs);
            updates.send(Update::Jobs(jobs)).ok();
            if has_arrays {
                match pbs::fetch_array_progress(user) {
                    Ok(text) => {
                        updates.send(Update::Array(Some(text))).ok();
                    }
                    Err(e) => {
                        updates
                            .send(Update::Error { source: "qarray".into(), message: e.to_string() })
                            .ok();
                    }
                }
            } else {
                updates.send(Update::Array(None)).ok();
            }
        }
        Err(e) => {
            updates
                .send(Update::Error { source: "qstat".into(), message: e.to_string() })
                .ok();
        }
    }
}

fn poll_cluster(highlight: &str, updates: &Sender<Update>) {
    match pbs::fetch_cluster_load(highlight) {
        Ok((text, users, userload)) => {
            updates.send(Update::Cluster(text)).ok();
            updates.send(Update::Users(users)).ok();
            updates.send(Update::UserLoad(userload)).ok();
        }
        Err(e) => {
            updates
                .send(Update::Error { source: "qlload".into(), message: e.to_string() })
                .ok();
        }
    }
}

pub fn spawn_poller(
    user: String,
    intervals: PollIntervals,
    updates: Sender<Update>,
) -> Sender<PollCmd> {
    let (cmd_tx, cmd_rx): (Sender<PollCmd>, Receiver<PollCmd>) = channel();

    std::thread::spawn(move || {
        let mut user = user; // mutable so `u`/-u can switch the monitored user live
        // Poll everything once at startup.
        poll_jobs(&user, &updates);
        poll_cluster(&user, &updates);
        let mut next_jobs = Instant::now() + intervals.jobs;
        let mut next_cluster = Instant::now() + intervals.cluster;

        loop {
            let now = Instant::now();
            let soonest = next_jobs.min(next_cluster);
            let wait = soonest.saturating_duration_since(now);

            match cmd_rx.recv_timeout(wait) {
                Ok(cmd) => {
                    // Refresh or user-switch: poll everything now, reset both deadlines.
                    if let PollCmd::SetUser(u) = &cmd {
                        user = u.clone();
                    }
                    poll_jobs(&user, &updates);
                    poll_cluster(&user, &updates);
                    next_jobs = Instant::now() + intervals.jobs;
                    next_cluster = Instant::now() + intervals.cluster;
                }
                Err(RecvTimeoutError::Timeout) => {
                    let now = Instant::now();
                    if now >= next_jobs {
                        poll_jobs(&user, &updates);
                        next_jobs = now + intervals.jobs;
                    }
                    if now >= next_cluster {
                        poll_cluster(&user, &updates);
                        next_cluster = now + intervals.cluster;
                    }
                }
                Err(RecvTimeoutError::Disconnected) => break,
            }
        }
    });

    cmd_tx
}
