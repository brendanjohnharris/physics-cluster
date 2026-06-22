use crate::app::Update;
use crate::pbs;
use std::sync::mpsc::{channel, Receiver, RecvTimeoutError, Sender};
use std::time::{Duration, Instant};

pub struct PollIntervals {
    pub jobs: Duration,
    pub cluster: Duration,
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

fn poll_cluster(updates: &Sender<Update>) {
    match pbs::fetch_cluster_load() {
        Ok(text) => {
            updates.send(Update::Cluster(text)).ok();
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
) -> Sender<()> {
    let (force_tx, force_rx): (Sender<()>, Receiver<()>) = channel();

    std::thread::spawn(move || {
        // Poll everything once at startup.
        poll_jobs(&user, &updates);
        poll_cluster(&updates);
        let mut next_jobs = Instant::now() + intervals.jobs;
        let mut next_cluster = Instant::now() + intervals.cluster;

        loop {
            let now = Instant::now();
            let soonest = next_jobs.min(next_cluster);
            let wait = soonest.saturating_duration_since(now);

            match force_rx.recv_timeout(wait) {
                Ok(()) => {
                    // Manual refresh: poll everything now and reset both deadlines.
                    poll_jobs(&user, &updates);
                    poll_cluster(&updates);
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
                        poll_cluster(&updates);
                        next_cluster = now + intervals.cluster;
                    }
                }
                Err(RecvTimeoutError::Disconnected) => break,
            }
        }
    });

    force_tx
}
