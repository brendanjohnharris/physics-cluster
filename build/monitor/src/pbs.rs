#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JobState {
    Running,
    Queued,
    Held,
    Other(char),
}

impl JobState {
    fn from_char(c: char) -> JobState {
        match c {
            'R' => JobState::Running,
            'Q' => JobState::Queued,
            'H' => JobState::Held,
            other => JobState::Other(other),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Job {
    pub id: String,
    pub owner: String,
    pub queue: String,
    pub name: String,
    pub state: JobState,
    pub node: Option<String>,
    pub cpus: Option<u32>,
    pub mem: Option<String>,
    pub req_walltime: Option<String>,
    pub elapsed: Option<String>,
}

/// True if the field is a job-id-shaped token: starts with a digit and the
/// only non-alphanumeric characters before the first '.' are '[' / ']'.
fn looks_like_job_id(field: &str) -> bool {
    let head = field.split('.').next().unwrap_or(field);
    let mut chars = head.chars();
    match chars.next() {
        Some(c) if c.is_ascii_digit() => {}
        _ => return false,
    }
    head.chars().all(|c| c.is_ascii_digit() || c == '[' || c == ']')
}

/// Numeric job number: digits before the first '[' or '.'.
pub fn job_number(id: &str) -> &str {
    let end = id.find(|c| c == '[' || c == '.').unwrap_or(id.len());
    &id[..end]
}

pub fn has_array_jobs(jobs: &[Job]) -> bool {
    jobs.iter().any(|j| j.id.contains('['))
}

/// Parse `qstat -u <user> -t -n1` output into jobs. Header/separator/blank
/// lines are skipped: only rows whose first field looks like a job id are kept.
pub fn parse_qstat(raw: &str) -> Vec<Job> {
    let mut jobs = Vec::new();
    for line in raw.lines() {
        let fields: Vec<&str> = line.split_whitespace().collect();
        if fields.len() < 10 || !looks_like_job_id(fields[0]) {
            continue;
        }
        let state_field = fields[9];
        if state_field.len() != 1 {
            continue;
        }
        let state = JobState::from_char(state_field.chars().next().unwrap());
        let node = fields.get(11).and_then(|n| {
            if *n == "--" {
                None
            } else {
                n.split('/').next().map(|h| h.to_string())
            }
        });
        let cpus = fields.get(6).and_then(|s| s.parse::<u32>().ok());
        let mem = fields.get(7).filter(|s| **s != "--").map(|s| s.to_string());
        let req_walltime = fields.get(8).filter(|s| **s != "--").map(|s| s.to_string());
        let elapsed = fields.get(10).filter(|s| **s != "--").map(|s| s.to_string());
        jobs.push(Job {
            id: fields[0].to_string(),
            owner: fields[1].to_string(),
            queue: fields[2].to_string(),
            name: fields[3].to_string(),
            state,
            node,
            cpus,
            mem,
            req_walltime,
            elapsed,
        });
    }
    jobs
}

use anyhow::{Context, Result};
use std::process::Command;

fn run(cmd: &str, args: &[&str], envs: &[(&str, &str)]) -> Result<String> {
    let mut c = Command::new(cmd);
    c.args(args);
    for (k, v) in envs {
        c.env(k, v);
    }
    let out = c
        .output()
        .with_context(|| format!("failed to spawn `{cmd}`"))?;
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr);
        anyhow::bail!("`{cmd}` exited {}: {}", out.status, err.trim());
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// True if we're running on the cluster headnode (where qstat/qarray work locally).
fn on_headnode() -> bool {
    std::fs::read_to_string("/proc/sys/kernel/hostname")
        .ok()
        .or_else(|| std::env::var("HOSTNAME").ok())
        .map(|h| h.trim().starts_with("headnode"))
        .unwrap_or(false)
}

/// Single-quote a string for safe inclusion in a remote shell command (protects
/// brackets in array job ids, etc.).
fn shell_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', r"'\''"))
}

/// Run a PBS client command: directly when on headnode, otherwise on headnode via
/// a (ControlMaster-multiplexed) SSH, since qstat/qarray are only configured there.
fn run_pbs(cmd: &str, args: &[&str]) -> Result<String> {
    if on_headnode() {
        run(cmd, args, &[])
    } else {
        let mut remote = shell_quote(cmd);
        for a in args {
            remote.push(' ');
            remote.push_str(&shell_quote(a));
        }
        run("ssh", &["-q", "-o", "LogLevel=QUIET", "headnode", &remote], &[])
    }
}

pub fn fetch_job_detail(jobid: &str) -> Result<String> {
    match run_pbs("qstat", &["-f", jobid]) {
        Ok(text) => Ok(text),
        Err(first_err) => {
            // Some PBS installations expose finished jobs only via -x history.
            if let Ok(text) = run_pbs("qstat", &["-xf", jobid]) {
                return Ok(text);
            }

            // Jobs can disappear between the jobs poll and details fetch.
            // Treat this as informational, not a hard UI error.
            let msg = first_err.to_string();
            // "Unknown Job" subsumes "Unknown Job Id"; keep the lowercase variant too.
            if msg.contains("Unknown Job") || msg.contains("unknown job id") {
                return Ok(format!(
                    "Job {jobid} is no longer available in the scheduler (it may have finished or been purged)."
                ));
            }

            Err(first_err)
        }
    }
}

/// Local file path from a `qstat -f` record's `Output_Path` attribute
/// (`host:/abs/path`), un-wrapping qstat's tab-continued lines and dropping the
/// `host:` prefix. Used as a log-preview fallback. `None` if the attribute is absent.
pub fn output_path(details: &str) -> Option<std::path::PathBuf> {
    let mut lines = details.lines();
    while let Some(line) = lines.next() {
        if let Some(first) = line.trim_start().strip_prefix("Output_Path = ") {
            let mut val = first.to_string();
            for cont in lines.by_ref() {
                match cont.strip_prefix('\t') {
                    Some(rest) => val.push_str(rest), // qstat wraps long values, tab-indented
                    None => break,
                }
            }
            let path = val.split_once(':').map(|(_, p)| p).unwrap_or(&val);
            return Some(std::path::PathBuf::from(path.trim()));
        }
    }
    None
}

pub fn fetch_user_jobs(user: &str) -> Result<Vec<Job>> {
    // Use wide mode so long job IDs are never truncated before details lookup.
    let raw = run_pbs("qstat", &["-w", "-u", user, "-t", "-n1"])?;
    Ok(parse_qstat(&raw))
}

/// Cluster load rendered by the in-process qlload port (see `qlload.rs`),
/// reverse-highlighting `highlight`'s load line and job markers. Fetches the two
/// PBS inputs over the same multiplexed SSH path as every other query, instead of
/// shelling out to the Python `qlload` (which re-SSHed to headnode per call).
/// Also returns every user with jobs on the cluster (for the `u` fuzzy switch) and
/// the structured per-user load (the ui draws that table so it can pack columns).
pub fn fetch_cluster_load(
    highlight: &str,
) -> Result<(String, Vec<String>, Vec<crate::qlload::UserLoad>)> {
    let jobs = run_pbs("qstat", &["-ft", "@headnode"])?;
    let nodes = run_pbs("pbsnodes", &["-av", "-s", "headnode"])?;
    Ok(crate::qlload::render_with_users(&jobs, &nodes, highlight))
}

/// Array-job progress, rendered by the in-process qarray port (see `qarray.rs`).
/// Finds the user's array masters, then fetches each one's subjobs (`qstat -t`) --- the
/// same calls the old bash `qarray` made, over the multiplexed SSH path.
pub fn fetch_array_progress(user: &str) -> Result<String> {
    let list = run_pbs("qstat", &["-w", "-u", user])?;
    let mut lines = Vec::new();
    for base in crate::qarray::array_bases(&list) {
        let detail = run_pbs("qstat", &["-t", &base])?;
        if let Some(line) = crate::qarray::render_array(&base, &detail) {
            lines.push(line);
        }
    }
    Ok(lines.join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "\nheadnode: \n                                                            Req'd  Req'd   Elap\nJob ID          Username Queue    Jobname    SessID NDS TSK Memory Time  S Time\n--------------- -------- -------- ---------- ------ --- --- ------ ----- - -----\n15992.headnode  bhar9988 l40s     code-phys* 669498   1  16   88gb 23:00 R 03:34 nodegpu02/3*0\n16001.headnode  bhar9988 batch    bigjob       --     1   8   32gb 10:00 Q   --    -- \n16050[3].headn  bhar9988 batch    arr*         --     1   1    4gb 02:00 R 00:05 cpunode01/2\n";

    #[test]
    fn parses_states_and_nodes() {
        let jobs = parse_qstat(SAMPLE);
        assert_eq!(jobs.len(), 3);

        assert_eq!(jobs[0].id, "15992.headnode");
        assert_eq!(jobs[0].state, JobState::Running);
        assert_eq!(jobs[0].node.as_deref(), Some("nodegpu02"));
        assert_eq!(jobs[0].owner, "bhar9988");
        assert_eq!(jobs[0].queue, "l40s");

        assert_eq!(jobs[1].state, JobState::Queued);
        assert_eq!(jobs[1].node, None);

        assert_eq!(jobs[2].state, JobState::Running);
        assert_eq!(jobs[2].node.as_deref(), Some("cpunode01"));
    }

    #[test]
    fn ignores_headers_and_blank_lines() {
        assert_eq!(parse_qstat("\n\nheadnode: \nJob ID Username\n--- ---\n").len(), 0);
    }

    #[test]
    fn detects_array_jobs() {
        let jobs = parse_qstat(SAMPLE);
        assert!(has_array_jobs(&jobs));
        assert_eq!(job_number("16050[3].headnode"), "16050");
        assert_eq!(job_number("15992.headnode"), "15992");
    }

    #[test]
    fn parses_output_path_unwrapping_continuation() {
        // qstat -f wraps long values onto tab-indented continuation lines.
        let d = "    Job_Name = x\n    Output_Path = head.usyd.edu.au:/head3/MD/Tasks/localh\n\tost/VASP.out\n    Priority = 0\n";
        assert_eq!(
            output_path(d),
            Some(std::path::PathBuf::from("/head3/MD/Tasks/localhost/VASP.out"))
        );
        assert_eq!(output_path("no attribute here"), None);
    }

    #[test]
    fn parses_resource_columns() {
        let jobs = parse_qstat(SAMPLE);
        assert_eq!(jobs[0].cpus, Some(16));
        assert_eq!(jobs[0].mem.as_deref(), Some("88gb"));
        assert_eq!(jobs[0].req_walltime.as_deref(), Some("23:00"));
        assert_eq!(jobs[0].elapsed.as_deref(), Some("03:34"));
        // queued job: Elap Time is "--", so elapsed is None
        assert_eq!(jobs[1].cpus, Some(8));
        assert_eq!(jobs[1].elapsed, None);
    }
}
