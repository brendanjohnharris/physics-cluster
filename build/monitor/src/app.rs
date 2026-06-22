use crate::pbs::{Job, JobState};
use std::time::Instant;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TopTab {
    SystemLoad,
    LogPreview,
    Details,
}

#[derive(Debug, Clone)]
pub enum Update {
    Jobs(Vec<Job>),
    Cluster(String),
    Array(Option<String>),
    Log { lines: Vec<String>, replace: bool },
    Error { source: String, message: String },
    Details { job: String, text: String },
}

pub struct App {
    pub jobs: Vec<Job>,
    pub running: Vec<usize>,          // indices into `jobs` with state Running
    pub selected: usize,              // index into `running`
    pub selected_job_id: Option<String>,
    pub cluster: String,
    pub cluster_at: Option<Instant>,
    pub array: Option<String>,
    pub jobs_at: Option<Instant>,
    pub log: Vec<String>,
    pub log_scroll: usize,
    pub log_follow: bool,
    pub cluster_follow: bool,
    pub details_follow: bool,
    pub last_error: Option<(String, Instant)>,
    pub should_quit: bool,
    pub start: Instant,
    pub top_tab: TopTab,
    pub cluster_scroll: usize,
    pub show_array: bool,
    pub show_queued: bool,
    pub show_running: bool,
    pub details: String,
    pub details_job: Option<String>,
    pub details_scroll: usize,
    pub top_inner_h: usize,
}

impl App {
    pub fn new() -> App {
        App {
            jobs: Vec::new(),
            running: Vec::new(),
            selected: 0,
            selected_job_id: None,
            cluster: String::new(),
            cluster_at: None,
            array: None,
            jobs_at: None,
            log: Vec::new(),
            log_scroll: 0,
            log_follow: true,
            cluster_follow: true,
            details_follow: true,
            last_error: None,
            should_quit: false,
            start: Instant::now(),
            top_tab: TopTab::SystemLoad,
            cluster_scroll: 0,
            show_array: true,
            show_queued: true,
            show_running: true,
            details: String::new(),
            details_job: None,
            details_scroll: 0,
            top_inner_h: 0,
        }
    }

    fn recompute_running(&mut self) {
        self.running = self
            .jobs
            .iter()
            .enumerate()
            .filter(|(_, j)| j.state == JobState::Running)
            .map(|(i, _)| i)
            .collect();
        // Re-anchor selection on the previously selected job id if still present.
        if let Some(id) = &self.selected_job_id {
            if let Some(pos) = self.running.iter().position(|&i| &self.jobs[i].id == id) {
                self.selected = pos;
                return;
            }
        }
        if self.selected >= self.running.len() {
            self.selected = 0;
        }
        self.selected_job_id = self.selected_job().map(|j| j.id.clone());
    }

    pub fn apply(&mut self, u: Update) {
        match u {
            Update::Jobs(jobs) => {
                self.jobs = jobs;
                self.recompute_running();
                self.jobs_at = Some(Instant::now());
            }
            Update::Cluster(text) => {
                self.cluster = text;
                self.cluster_at = Some(Instant::now());
            }
            Update::Array(a) => self.array = a,
            Update::Log { lines, replace } => {
                if replace {
                    self.log = lines;
                } else {
                    self.log.extend(lines);
                }
                let cap = 5000;
                if self.log.len() > cap {
                    let drop = self.log.len() - cap;
                    self.log.drain(0..drop);
                }
            }
            Update::Error { source, message } => {
                self.last_error = Some((format!("{source}: {message}"), Instant::now()));
            }
            Update::Details { job, text } => {
                self.details = text;
                self.details_job = Some(job);
                self.details_scroll = 0;
            }
        }
    }

    pub fn next_job(&mut self) {
        if self.running.is_empty() {
            return;
        }
        self.selected = (self.selected + 1) % self.running.len();
        self.selected_job_id = self.selected_job().map(|j| j.id.clone());
        self.log_follow = true;
        self.log_scroll = 0;
    }

    pub fn prev_job(&mut self) {
        if self.running.is_empty() {
            return;
        }
        self.selected = (self.selected + self.running.len() - 1) % self.running.len();
        self.selected_job_id = self.selected_job().map(|j| j.id.clone());
        self.log_follow = true;
        self.log_scroll = 0;
    }

    pub fn selected_job(&self) -> Option<&Job> {
        self.running.get(self.selected).map(|&i| &self.jobs[i])
    }

    pub fn running_count(&self) -> usize {
        self.running.len()
    }

    pub fn selected_ordinal(&self) -> usize {
        if self.running.is_empty() {
            0
        } else {
            self.selected + 1
        }
    }

    pub fn next_tab(&mut self) {
        self.top_tab = match self.top_tab {
            TopTab::SystemLoad => TopTab::LogPreview,
            TopTab::LogPreview => TopTab::Details,
            TopTab::Details => TopTab::SystemLoad,
        };
    }

    pub fn prev_tab(&mut self) {
        self.top_tab = match self.top_tab {
            TopTab::SystemLoad => TopTab::Details,
            TopTab::LogPreview => TopTab::SystemLoad,
            TopTab::Details => TopTab::LogPreview,
        };
    }

    fn active_follow(&mut self) -> &mut bool {
        match self.top_tab {
            TopTab::SystemLoad => &mut self.cluster_follow,
            TopTab::LogPreview => &mut self.log_follow,
            TopTab::Details => &mut self.details_follow,
        }
    }

    fn active_scroll(&mut self) -> &mut usize {
        match self.top_tab {
            TopTab::SystemLoad => &mut self.cluster_scroll,
            TopTab::LogPreview => &mut self.log_scroll,
            TopTab::Details => &mut self.details_scroll,
        }
    }

    pub fn page_up(&mut self) {
        let step = self.top_inner_h.max(1);
        *self.active_follow() = false;
        let s = self.active_scroll();
        *s = s.saturating_sub(step);
    }

    pub fn page_down(&mut self) {
        let step = self.top_inner_h.max(1);
        *self.active_follow() = false;
        let s = self.active_scroll();
        *s = s.saturating_add(step);
    }

    pub fn scroll_up(&mut self) {
        *self.active_follow() = false;
        let s = self.active_scroll();
        *s = s.saturating_sub(1);
    }

    pub fn scroll_down(&mut self) {
        *self.active_follow() = false;
        let s = self.active_scroll();
        *s = s.saturating_add(1);
    }

    pub fn scroll_to_top(&mut self) {
        *self.active_follow() = false;
        *self.active_scroll() = 0;
    }

    pub fn scroll_to_bottom(&mut self) {
        *self.active_follow() = true;
    }

    pub fn toggle_array(&mut self) {
        self.show_array = !self.show_array;
    }

    pub fn toggle_queued(&mut self) {
        self.show_queued = !self.show_queued;
    }

    pub fn toggle_running(&mut self) {
        self.show_running = !self.show_running;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn job(id: &str, state: JobState) -> Job {
        Job { id: id.into(), owner: "u".into(), queue: "q".into(), name: "n".into(), state, node: None, cpus: None, mem: None, req_walltime: None, elapsed: None }
    }

    #[test]
    fn tracks_running_jobs_and_wraps() {
        let mut app = App::new();
        app.apply(Update::Jobs(vec![
            job("1.h", JobState::Running),
            job("2.h", JobState::Queued),
            job("3.h", JobState::Running),
        ]));
        assert_eq!(app.running_count(), 2);
        assert_eq!(app.selected_job().unwrap().id, "1.h");
        app.next_job();
        assert_eq!(app.selected_job().unwrap().id, "3.h");
        app.next_job(); // wrap
        assert_eq!(app.selected_job().unwrap().id, "1.h");
        app.prev_job(); // wrap back
        assert_eq!(app.selected_job().unwrap().id, "3.h");
    }

    #[test]
    fn keeps_selection_across_refresh_when_job_persists() {
        let mut app = App::new();
        app.apply(Update::Jobs(vec![job("1.h", JobState::Running), job("3.h", JobState::Running)]));
        app.next_job();
        assert_eq!(app.selected_job().unwrap().id, "3.h");
        // refresh: job 1 finished, 3 remains, 5 appears
        app.apply(Update::Jobs(vec![job("3.h", JobState::Running), job("5.h", JobState::Running)]));
        assert_eq!(app.selected_job().unwrap().id, "3.h");
    }

    #[test]
    fn log_replace_then_append() {
        let mut app = App::new();
        app.apply(Update::Log { lines: vec!["a".into(), "b".into()], replace: true });
        app.apply(Update::Log { lines: vec!["c".into()], replace: false });
        assert_eq!(app.log, vec!["a", "b", "c"]);
    }

    #[test]
    fn tabs_cycle_three_ways() {
        let mut app = App::new();
        assert_eq!(app.top_tab, TopTab::SystemLoad);
        app.next_tab();
        assert_eq!(app.top_tab, TopTab::LogPreview);
        app.next_tab();
        assert_eq!(app.top_tab, TopTab::Details);
        app.next_tab();
        assert_eq!(app.top_tab, TopTab::SystemLoad);
        app.prev_tab();
        assert_eq!(app.top_tab, TopTab::Details);
        app.prev_tab();
        assert_eq!(app.top_tab, TopTab::LogPreview);
    }

    #[test]
    fn scroll_acts_on_active_tab() {
        let mut app = App::new();
        // System Load tab active: scroll affects cluster_scroll and clears its follow.
        app.scroll_down();
        assert_eq!(app.cluster_scroll, 1);
        assert_eq!(app.log_scroll, 0);
        assert!(!app.cluster_follow);
        app.scroll_up();
        app.scroll_up(); // saturates at 0
        assert_eq!(app.cluster_scroll, 0);
        // Switch to Log tab: scroll now affects log_scroll and clears log_follow.
        app.next_tab();
        app.scroll_down();
        assert_eq!(app.log_scroll, 1);
        assert!(!app.log_follow);
        assert_eq!(app.cluster_scroll, 0);
        // End re-engages follow on the active tab.
        app.scroll_to_bottom();
        assert!(app.log_follow);
    }

    #[test]
    fn details_update_and_scroll() {
        let mut app = App::new();
        app.apply(Update::Details {
            job: "15992.headnode".into(),
            text: "a\nb\nc".into(),
        });
        assert_eq!(app.details_job.as_deref(), Some("15992.headnode"));
        assert!(app.details.contains('b'));
        app.next_tab(); // SystemLoad -> LogPreview
        app.next_tab(); // LogPreview -> Details
        assert_eq!(app.top_tab, TopTab::Details);
        app.scroll_down();
        assert_eq!(app.details_scroll, 1);
        app.scroll_to_top();
        assert_eq!(app.details_scroll, 0);
    }

    #[test]
    fn page_scroll_uses_viewport_height() {
        let mut app = App::new();
        app.top_inner_h = 10; // as if a render set the page height
        app.page_down();
        assert_eq!(app.cluster_scroll, 10);
        assert!(!app.cluster_follow);
        app.page_up();
        assert_eq!(app.cluster_scroll, 0);
    }

    #[test]
    fn section_toggles_flip_flags() {
        let mut app = App::new();
        assert!(app.show_array && app.show_queued && app.show_running);
        app.toggle_array();
        app.toggle_queued();
        app.toggle_running();
        assert!(!app.show_array && !app.show_queued && !app.show_running);
    }
}
