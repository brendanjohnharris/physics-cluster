//! In-process reimplementation of the `qlload` PBS resource monitor.
//!
//! Ports the Python `~/.local/bin/qlload` (v2.1) into monitor so the cluster-load
//! panel no longer shells out to a script that re-SSHes to headnode on every poll.
//! Data is fetched through `pbs::run_pbs` (multiplexed SSH) and rendered here into
//! the same ANSI-coloured text the panel already knows how to display via
//! `ansi-to-tui`. The one behavioural change from the script: the highlighted user
//! is a parameter (`highlight`), so `monitor -u <user>` and the live `u` switch
//! reverse-video the requested user's load line and per-core job markers.
//!
//! Faithful to the script's output except: the `finger` full-name lookup is dropped
//! (it is not installed on headnode, so names did not render anyway), and the
//! unused `-q`/`-a`/cluster-stats code paths are not ported.

use std::collections::HashMap;

// ANSI codes, matching the Python `Colors` class verbatim.
const RESET: &str = "\x1b[0m";
const REVERSE: &str = "\x1b[7m";
const RED: &str = "\x1b[31m";
const GREEN: &str = "\x1b[32m";
const YELLOW: &str = "\x1b[33m";

/// Per-user colours assigned in order of first appearance, so a user keeps one
/// colour across both the load list and the node markers within a single render.
const USER_COLORS: [&str; 12] = [
    "\x1b[31m", "\x1b[32m", "\x1b[33m", "\x1b[34m", "\x1b[35m", "\x1b[36m", // plain
    "\x1b[4;31m", "\x1b[4;32m", "\x1b[4;33m", "\x1b[4;34m", "\x1b[4;35m", "\x1b[4;36m", // underlined
];

/// Assigns each user a stable colour *index* (0-based, order of first appearance).
/// The index maps to an ANSI code here and to a matching ratatui colour in `ui.rs`,
/// so a user looks the same in the (ui-rendered) load table and the (ANSI) node graph.
struct Palette {
    map: HashMap<String, usize>,
    next: usize,
}
impl Palette {
    fn new() -> Palette {
        Palette { map: HashMap::new(), next: 0 }
    }
    fn index(&mut self, user: &str) -> usize {
        if let Some(&i) = self.map.get(user) {
            return i;
        }
        let i = self.next;
        self.next += 1;
        self.map.insert(user.to_string(), i);
        i
    }
}

/// ANSI escape for a palette colour index (wraps every 12).
fn ansi_for(idx: usize) -> &'static str {
    USER_COLORS[idx % USER_COLORS.len()]
}

fn wrap(s: &str, color: &str) -> String {
    format!("{color}{s}{RESET}")
}

/// One user's cluster usage, for the ui-rendered "User load" table. `color` is the
/// palette index (see `Palette`); `secs` is the largest remaining walltime among the
/// user's running jobs, `None` if they have none running.
#[derive(Debug, Clone)]
pub struct UserLoad {
    pub name: String,
    pub cores: i64,
    pub pending: i64,
    pub secs: Option<i64>,
    pub color: u8,
    pub highlighted: bool,
}

/// A parsed `qstat -ft` job record (only the fields qlload actually uses).
#[derive(Debug, Clone)]
pub struct Job {
    pub number: String, // job number: digits (+ `[task]`) before the first '.'
    pub user: String,
    pub state: char, // PBS single-letter state (R, Q, ...)
    pub cpus: i64,
    pub sec_remaining: i64, // req - used walltime for running jobs; -1 if unknown
}

/// "HH:MM:SS" (H may exceed 99) to seconds.
fn wall_to_sec(s: &str) -> Option<i64> {
    let mut it = s.split(':');
    let h: i64 = it.next()?.trim().parse().ok()?;
    let m: i64 = it.next()?.trim().parse().ok()?;
    let sec: i64 = it.next()?.trim().parse().ok()?;
    Some(h * 3600 + m * 60 + sec)
}

/// Value of an indented `key = value` line, if this line is exactly that key.
fn field<'a>(line: &'a str, key: &str) -> Option<&'a str> {
    let t = line.trim_start();
    t.strip_prefix(key).and_then(|r| r.strip_prefix(" = ")).map(|v| v.trim())
}

/// Parse `qstat -ft` output. Records begin at a `Job Id:` line and run until the
/// next one; fields are matched by exact key so wrapped continuation lines (e.g.
/// Variable_List) are ignored.
pub fn parse_jobs(raw: &str) -> Vec<Job> {
    let mut jobs = Vec::new();
    let mut number: Option<String> = None;
    let mut user = String::new();
    let mut state = ' ';
    let mut cpus: i64 = 0;
    let mut req: Option<i64> = None;
    let mut used: Option<i64> = None;

    let mut flush = |number: &mut Option<String>,
                     user: &mut String,
                     state: &mut char,
                     cpus: &mut i64,
                     req: &mut Option<i64>,
                     used: &mut Option<i64>| {
        if let Some(num) = number.take() {
            // Precedence mirrors the Python Job.__init__ walltime logic.
            let sec_remaining = match (*state == 'R', *req, *used) {
                (true, Some(r), Some(u)) => r - u,
                (_, Some(r), _) => r,
                _ => -1,
            };
            jobs.push(Job {
                number: num,
                user: std::mem::take(user),
                state: *state,
                cpus: *cpus,
                sec_remaining,
            });
        }
        *state = ' ';
        *cpus = 0;
        *req = None;
        *used = None;
    };

    for line in raw.lines() {
        if let Some(rest) = line.strip_prefix("Job Id:") {
            flush(&mut number, &mut user, &mut state, &mut cpus, &mut req, &mut used);
            let id = rest.trim();
            number = Some(id.split('.').next().unwrap_or(id).to_string());
        } else if let Some(v) = field(line, "Job_Owner") {
            user = v.split('@').next().unwrap_or(v).to_string();
        } else if let Some(v) = field(line, "job_state") {
            state = v.chars().next().unwrap_or(' ');
        } else if let Some(v) = field(line, "Resource_List.ncpus") {
            cpus = v.parse().unwrap_or(0);
        } else if let Some(v) = field(line, "Resource_List.walltime") {
            req = wall_to_sec(v);
        } else if let Some(v) = field(line, "resources_used.walltime") {
            used = wall_to_sec(v);
        }
    }
    flush(&mut number, &mut user, &mut state, &mut cpus, &mut req, &mut used);
    jobs
}

/// A parsed `pbsnodes -av` vnode record.
#[derive(Debug, Clone)]
pub struct Node {
    pub name: String,
    pub state: String,
    pub jobs: Vec<String>, // job numbers of headnode jobs on this node
    pub nproc: i64,
    pub availmem: String, // raw resources_available.mem (e.g. "63437mb")
    pub usedmem: String,  // raw resources_assigned.mem
    pub ngpus: i64,
    pub usedgpus: i64,
}

/// Extract headnode job numbers from a `jobs = ...` value: `19657[62].headnode/0`
/// -> `19657[62]`, keeping only entries that name headnode.
fn parse_node_jobs(v: &str) -> Vec<String> {
    v.split(',')
        .map(|t| t.trim())
        .filter(|t| t.contains("headnode"))
        .filter_map(|t| t.split('/').next())
        .filter_map(|t| t.split('.').next())
        .map(|s| s.to_string())
        .collect()
}

/// Parse `pbsnodes -av` output. Records are blank-line separated; only vnode
/// records (whose name line contains `[`) are kept, matching the Python.
pub fn parse_nodes(raw: &str) -> Vec<Node> {
    let mut nodes = Vec::new();
    for block in raw.split("\n\n") {
        let mut lines = block.lines();
        let name = match lines.next() {
            Some(l) => l.trim().to_string(),
            None => continue,
        };
        if !name.contains('[') {
            continue;
        }
        let mut node = Node {
            name,
            state: String::new(),
            jobs: Vec::new(),
            nproc: 0,
            availmem: String::new(),
            usedmem: String::new(),
            ngpus: 0,
            usedgpus: 0,
        };
        for line in lines {
            if let Some(v) = field(line, "state") {
                node.state = v.to_string();
            } else if let Some(v) = field(line, "jobs") {
                node.jobs = parse_node_jobs(v);
            } else if let Some(v) = field(line, "resources_available.ncpus") {
                node.nproc = v.parse().unwrap_or(0);
            } else if let Some(v) = field(line, "resources_available.mem") {
                node.availmem = v.to_string();
            } else if let Some(v) = field(line, "resources_assigned.mem") {
                node.usedmem = v.to_string();
            } else if let Some(v) = field(line, "resources_available.ngpus") {
                node.ngpus = v.parse().unwrap_or(0);
            } else if let Some(v) = field(line, "resources_assigned.ngpus") {
                node.usedgpus = v.parse().unwrap_or(0);
            }
        }
        nodes.push(node);
    }
    nodes
}

/// A node has usable capacity worth graphing (free or busy, not down/offline).
fn is_renderable(state: &str) -> bool {
    (state.contains("free") || state.contains("job-busy")) && !state.contains("offline")
}

/// Combine a physical host's vnodes (`cmt01[0]`, `cmt01[1]`, ...) into one row named
/// `cmt01`, summing cores, jobs, memory (normalised to kb), and GPUs. The merged row
/// is renderable if any vnode is. First-appearance order preserved.
fn merge_vnodes(nodes: Vec<Node>) -> Vec<Node> {
    let mut order: Vec<String> = Vec::new();
    let mut by_host: HashMap<String, Node> = HashMap::new();
    for n in nodes {
        let host = n.name.split('[').next().unwrap_or(&n.name).to_string();
        let avail = parse_mem(&n.availmem).unwrap_or(0.0);
        let used = parse_mem(&n.usedmem).unwrap_or(0.0);
        match by_host.get_mut(&host) {
            Some(acc) => {
                acc.nproc += n.nproc;
                acc.ngpus += n.ngpus;
                acc.usedgpus += n.usedgpus;
                acc.jobs.extend(n.jobs);
                acc.availmem = format!("{}kb", parse_mem(&acc.availmem).unwrap_or(0.0) + avail);
                acc.usedmem = format!("{}kb", parse_mem(&acc.usedmem).unwrap_or(0.0) + used);
                if is_renderable(&n.state) && !is_renderable(&acc.state) {
                    acc.state = n.state; // any renderable vnode makes the host renderable
                }
            }
            None => {
                order.push(host.clone());
                let mut base = n;
                base.name = host.clone();
                base.availmem = format!("{avail}kb"); // normalise for later summing
                base.usedmem = format!("{used}kb");
                by_host.insert(host, base);
            }
        }
    }
    order.into_iter().filter_map(|h| by_host.remove(&h)).collect()
}

/// Memory string ("1024kb", "2gb", "512991mb", "5b") to KB.
fn parse_mem(s: &str) -> Option<f64> {
    let m = s.trim().to_lowercase();
    if let Some(v) = m.strip_suffix("kb") {
        v.trim().parse().ok()
    } else if let Some(v) = m.strip_suffix("mb") {
        v.trim().parse::<f64>().ok().map(|x| x * 1024.0)
    } else if let Some(v) = m.strip_suffix("gb") {
        v.trim().parse::<f64>().ok().map(|x| x * 1024.0 * 1024.0)
    } else if let Some(v) = m.strip_suffix('b') {
        v.trim().parse::<f64>().ok().map(|x| x / 1024.0)
    } else if m.is_empty() {
        None
    } else {
        m.parse().ok() // assume KB
    }
}

/// (memstr, free-percent) for a node's assigned-vs-available memory.
fn mem_info(n: &Node) -> (String, f64) {
    match (parse_mem(&n.availmem), parse_mem(&n.usedmem)) {
        (Some(avail_kb), Some(used_kb)) => {
            let avail_gb = avail_kb / 1048576.0;
            let used_gb = used_kb / 1048576.0;
            let free_gb = avail_gb - used_gb;
            let pct = if avail_kb > 0.0 { free_gb / avail_gb * 100.0 } else { 0.0 };
            if avail_gb > 0.0 {
                (format!("{}/{}GB", free_gb as i64, avail_gb as i64), pct)
            } else {
                ("NO MEM".to_string(), pct)
            }
        }
        _ => ("Err".to_string(), 0.0),
    }
}

/// Append a graph glyph, coloured by the job marker at `marker_idx` when that column
/// is in use (the marker carries the owning user's ANSI colour; `•` -> the glyph).
fn push_glyph(g: &mut String, ch: &str, markers: &[String], marker_idx: usize, filled: bool) {
    match markers.get(marker_idx) {
        Some(m) if filled && m.starts_with('\u{1b}') => g.push_str(&m.replace('•', ch)),
        _ => g.push_str(ch),
    }
}

/// Two-row Braille CPU graph for mid-size nodes (33-64 cores; each column = 2 cores):
/// `∶` both used, `⠄` one used, `=` free.
fn braille_graph(nproc: i64, load: i64, markers: &[String]) -> String {
    let width = ((nproc + 1) / 2) as usize;
    let mut g = String::from("|");
    for i in 0..width {
        let base = (i * 2) as i64;
        let cores = if base + 1 < nproc { 2 } else { 1 };
        let filled = cores.min((load - base).max(0));
        let ch = if filled == 2 { "\u{2236}" } else if filled == 1 { "⠄" } else { "=" }; // ∶ / ⠄ / =
        push_glyph(&mut g, ch, markers, i * 2, filled > 0);
    }
    g.push('|');
    g
}

/// CPU graph for large nodes, `per_col` cores per column: `used` if any core in the
/// column is busy, `free` if all are idle. Covers the triple (`⋮`/`≡`, 65-96 cores)
/// and quad (`⸬`/`≣`, >96) tiers.
fn tier_graph(nproc: i64, load: i64, markers: &[String], per_col: i64, used: &str, free: &str) -> String {
    let width = ((nproc + per_col - 1) / per_col) as usize;
    let mut g = String::from("|");
    for i in 0..width {
        let base = i as i64 * per_col;
        let cores = (nproc - base).min(per_col);
        let filled = cores.min((load - base).max(0));
        let ch = if filled > 0 { used } else { free };
        push_glyph(&mut g, ch, markers, i * per_col as usize, filled > 0);
    }
    g.push('|');
    g
}

/// Render one node's indicator columns: cpu free, mem free, gpu, and the CPU graph.
fn node_indicator(
    n: &Node,
    jmap: &HashMap<&str, &str>,
    pal: &mut Palette,
    highlight: &str,
) -> String {
    if !is_renderable(&n.state) {
        return "down/offline".to_string();
    }

    // One coloured bullet per job on the node; the highlighted user's are reversed.
    let markers: Vec<String> = n
        .jobs
        .iter()
        .map(|j| match jmap.get(j.as_str()) {
            Some(&user) => {
                let c = ansi_for(pal.index(user));
                if user == highlight {
                    format!("{c}{REVERSE}•{RESET}")
                } else {
                    format!("{c}•{RESET}")
                }
            }
            None => "•".to_string(),
        })
        .collect();

    let load = n.jobs.len() as i64;
    let nproc = n.nproc;

    // Glyph density scales with node size: single dots (<=32), Braille 2-up (33-64),
    // triple lines (65-96), four-per-column (>96).
    let graph = if nproc > 96 {
        tier_graph(nproc, load, &markers, 4, "\u{2e2c}", "\u{2263}") // ⸬ / ≣ (BMP, mobile-safe)
    } else if nproc > 64 {
        tier_graph(nproc, load, &markers, 3, "\u{22ee}", "\u{2261}") // ⋮ / ≡
    } else if nproc > 32 {
        braille_graph(nproc, load, &markers)
    } else {
        let mut g = String::from("|");
        for m in &markers {
            g.push_str(m);
        }
        for _ in 0..(nproc - load).max(0) {
            g.push('-');
        }
        g.push('|');
        g
    };

    // Fixed-width columns (right-justified) so rows align regardless of digit count.
    let util = if nproc > 0 { load as f64 / nproc as f64 * 100.0 } else { 0.0 };
    let cpu = wrap(
        &format!("{:>8}", format!("{}/{}", nproc - load, nproc)),
        if util >= 80.0 { RED } else if util >= 50.0 { YELLOW } else { GREEN },
    );

    let (memstr, mem_pct) = mem_info(n);
    let mem_fmt = format!("{:>11}", memstr);
    let mem = if memstr == "Err" {
        mem_fmt
    } else if memstr == "NO MEM" {
        wrap(&mem_fmt, RED)
    } else {
        wrap(
            &mem_fmt,
            if mem_pct >= 50.0 { GREEN } else if mem_pct >= 20.0 { YELLOW } else { RED },
        )
    };

    let gpu = if n.ngpus > 0 {
        let free = n.ngpus - n.usedgpus;
        let pct = free as f64 / n.ngpus as f64 * 100.0;
        wrap(
            &format!("{:>5}", format!("{free}/{}", n.ngpus)),
            if pct >= 50.0 { GREEN } else if pct > 0.0 { YELLOW } else { RED },
        )
    } else {
        " ".repeat(5)
    };

    format!("{cpu}  {mem}  {gpu}  {graph}")
}

/// Sorted, unique usernames appearing in the job list (any state) --- the
/// candidate set for the `u` fuzzy user switch.
fn users_of(jobs: &[Job]) -> Vec<String> {
    let mut u: Vec<String> = jobs.iter().map(|j| j.user.clone()).collect();
    u.sort();
    u.dedup();
    u
}

/// Per-user [running, pending] core counts, sorted running-descending, each tagged
/// with a palette colour index (assigned in this order so it seeds `pal` before the
/// node markers reuse it) and the largest remaining walltime among running jobs.
fn compute_userload(jobs: &[Job], highlight: &str, pal: &mut Palette) -> Vec<UserLoad> {
    let mut load: HashMap<&str, [i64; 2]> = HashMap::new();
    for j in jobs {
        match j.state {
            'R' => load.entry(&j.user).or_insert([0, 0])[0] += j.cpus,
            'Q' => load.entry(&j.user).or_insert([0, 0])[1] += j.cpus,
            _ => {}
        }
    }
    let mut items: Vec<(&str, [i64; 2])> = load.iter().map(|(k, v)| (*k, *v)).collect();
    items.sort_by(|a, b| b.1[0].cmp(&a.1[0]).then(a.0.cmp(b.0))); // running desc, name tiebreak
    items
        .into_iter()
        .map(|(user, cnt)| UserLoad {
            name: user.to_string(),
            cores: cnt[0],
            pending: cnt[1],
            secs: jobs
                .iter()
                .filter(|j| j.state == 'R' && j.user == user)
                .map(|j| j.sec_remaining)
                .max(),
            color: pal.index(user) as u8,
            highlighted: user == highlight,
        })
        .collect()
}

/// Render the ANSI node graph and return the cluster user list + structured user
/// load. Parses `qstat -ft` once. The user-load table itself is drawn by `ui.rs`
/// (so it can pack columns to the terminal width); colours stay consistent because
/// `compute_userload` seeds the shared palette before the node markers reuse it.
pub fn render_with_users(
    qstat_ft: &str,
    pbsnodes: &str,
    highlight: &str,
) -> (String, Vec<String>, Vec<UserLoad>) {
    let jobs = parse_jobs(qstat_ft);
    let nodes = merge_vnodes(parse_nodes(pbsnodes)); // one row per physical host
    let users = users_of(&jobs);
    let mut pal = Palette::new();
    let userload = compute_userload(&jobs, highlight, &mut pal);
    let graph = render_node_graph(&jobs, &nodes, highlight, &mut pal);
    (graph, users, userload)
}

/// The node-graph ANSI (used by tests; the panel proper also has the ui user table).
pub fn render(qstat_ft: &str, pbsnodes: &str, highlight: &str) -> String {
    render_with_users(qstat_ft, pbsnodes, highlight).0
}

/// Formatted "User load" table cells with aligned USER / CORES / QUEUED / REMAINING
/// sub-columns: the bold header cell plus one `(cell, colour index, highlighted)` per
/// user. Shared by the TUI (`ui::user_load_lines`) and the standalone `qlload`
/// (`render_userload_ansi`) so the column layout lives in one place.
pub fn userload_cells(userload: &[UserLoad]) -> (String, Vec<(String, u8, bool)>) {
    let cores = |u: &UserLoad| u.cores.to_string();
    let queued = |u: &UserLoad| {
        if u.pending > 0 {
            u.pending.to_string()
        } else {
            String::new()
        }
    };
    let time = |u: &UserLoad| match u.secs {
        None => "--".to_string(),
        Some(s) if s < 0 => "?".to_string(),
        Some(s) => format!("{}h {:02}m", s / 3600, (s % 3600) / 60),
    };
    let uw = userload.iter().map(|u| u.name.chars().count()).max().unwrap_or(0).max(4);
    let cw = userload.iter().map(|u| cores(u).len()).max().unwrap_or(0).max(5);
    let qw = userload.iter().map(|u| queued(u).len()).max().unwrap_or(0).max(6);
    let tw = userload.iter().map(|u| time(u).chars().count()).max().unwrap_or(0).max(9);
    let cell = |name: &str, c: &str, q: &str, t: &str| {
        format!("{:<uw$}  {:>cw$}  {:>qw$}  {:>tw$}", name, c, q, t, uw = uw, cw = cw, qw = qw, tw = tw)
    };
    let header = cell("USER", "CORES", "QUEUED", "REMAINING");
    let rows = userload
        .iter()
        .map(|u| (cell(&u.name, &cores(u), &queued(u), &time(u)), u.color, u.highlighted))
        .collect();
    (header, rows)
}

/// Single-column ANSI "User load" table for the standalone `qlload` command (the TUI
/// draws its own width-packed version in `ui.rs`).
pub fn render_userload_ansi(userload: &[UserLoad]) -> String {
    if userload.is_empty() {
        return String::new();
    }
    let (header, rows) = userload_cells(userload);
    let mut out = format!("\u{1b}[1m{header}\u{1b}[0m\n");
    for (row, color, highlighted) in rows {
        let c = ansi_for(color as usize);
        out.push_str(&if highlighted {
            format!("{c}{REVERSE}{row}{RESET}\n")
        } else {
            format!("{c}{row}{RESET}\n")
        });
    }
    out
}

fn render_node_graph(jobs: &[Job], nodes: &[Node], highlight: &str, pal: &mut Palette) -> String {
    let jmap: HashMap<&str, &str> =
        jobs.iter().map(|j| (j.number.as_str(), j.user.as_str())).collect();
    let mut out = String::new();
    // Bold, upper-case header aligned to the fixed columns (see node_indicator).
    out.push_str(&format!(
        "\u{1b}[1m{:<12} {:>8}  {:>11}  {:>5}  {}\u{1b}[0m\n",
        "NODE", "CPU", "MEM", "GPU", "GRAPH"
    ));
    let mut ns: Vec<&Node> = nodes.iter().collect();
    ns.sort_by(|a, b| a.nproc.cmp(&b.nproc).then(a.name.cmp(&b.name)));
    for n in ns {
        out.push_str(&format!("{:<12} {}\n", n.name, node_indicator(n, &jmap, pal, highlight)));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const JOBS: &str = "\
Job Id: 18197.headnode
    Job_Name = T-VASP
    Job_Owner = medea@headnode.physics.usyd.edu.au
    resources_used.walltime = 100:00:00
    job_state = R
    queue = jasper_cpu
    Resource_List.ncpus = 42
    Resource_List.walltime = 350:00:00

Job Id: 21173.headnode
    Job_Owner = bhar9988@headnode.physics.usyd.edu.au
    job_state = R
    Resource_List.ncpus = 16
    Resource_List.walltime = 23:00:00
    resources_used.walltime = 03:00:00

Job Id: 21200.headnode
    Job_Owner = bhar9988@headnode.physics.usyd.edu.au
    job_state = Q
    Resource_List.ncpus = 8
    Resource_List.walltime = 10:00:00
";

    const NODES: &str = "\
nodegpu02[2]
     state = free
     jobs = 21173.headnode/0, 19657[62].headnode/1
     resources_available.mem = 63437mb
     resources_available.ncpus = 16
     resources_available.ngpus = 2
     resources_assigned.mem = 30000mb
     resources_assigned.ngpus = 1

cmt01[0]
     state = job-busy
     jobs = 18197.headnode/0
     resources_available.mem = 512991mb
     resources_available.ncpus = 84
     resources_available.ngpus = 0
     resources_assigned.mem = 100000mb
     resources_assigned.ngpus = 0

downnode[0]
     state = down,offline
     resources_available.ncpus = 8
";

    #[test]
    fn parses_jobs_states_cpus_and_walltime() {
        let jobs = parse_jobs(JOBS);
        assert_eq!(jobs.len(), 3);
        assert_eq!(jobs[0].number, "18197");
        assert_eq!(jobs[0].user, "medea");
        assert_eq!(jobs[0].state, 'R');
        assert_eq!(jobs[0].cpus, 42);
        // 350h req - 100h used = 250h = 900000s
        assert_eq!(jobs[0].sec_remaining, 900000);
        assert_eq!(jobs[2].state, 'Q');
    }

    #[test]
    fn parses_array_task_job_number() {
        // Node markers use the same "digits[task]" number as the qstat job number.
        let nodes = parse_nodes(NODES);
        assert_eq!(nodes[0].jobs, vec!["21173", "19657[62]"]);
    }

    #[test]
    fn skips_non_vnode_and_keeps_bracketed() {
        let nodes = parse_nodes(NODES);
        assert_eq!(nodes.len(), 3);
        assert!(nodes.iter().all(|n| n.name.contains('[')));
    }

    #[test]
    fn compute_userload_sorts_colors_and_times() {
        let jobs = parse_jobs(JOBS);
        let mut pal = Palette::new();
        let ul = compute_userload(&jobs, "bhar9988", &mut pal);
        // medea has 42 running (most) -> first, colour index 0.
        assert_eq!(ul[0].name, "medea");
        assert_eq!(ul[0].cores, 42);
        assert_eq!(ul[0].color, 0);
        assert_eq!(ul[0].secs, Some(900000)); // 350h req - 100h used
        // bhar9988: 16 running, 8 pending, highlighted.
        assert_eq!(ul[1].name, "bhar9988");
        assert_eq!(ul[1].cores, 16);
        assert_eq!(ul[1].pending, 8);
        assert!(ul[1].highlighted);
    }

    #[test]
    fn render_node_graph_tiers_and_merged_names() {
        let out = render(JOBS, NODES, "bhar9988");
        // vnodes are merged: names drop the [n] suffix.
        assert!(out.contains("nodegpu02") && !out.contains("nodegpu02["));
        assert!(out.contains("cmt01") && !out.contains("cmt01["));
        assert!(out.contains("downnode") && !out.contains("downnode["));
        // 16-core node: single-row graph (centre dots / dashes).
        assert!(out.contains('•') || out.contains('-'));
        // 84-core node: triple-line graph (⋮ filled, ≡ empty).
        assert!(out.contains('\u{22ee}') && out.contains('\u{2261}'));
        // down node shows down/offline, no graph.
        assert!(out.contains("down/offline"));
    }

    #[test]
    fn merge_vnodes_combines_host_and_sums() {
        let raw = "\
host9[0]
     state = free
     jobs = 100.headnode/0
     resources_available.mem = 100000mb
     resources_available.ncpus = 40
     resources_assigned.mem = 0kb

host9[1]
     state = free
     jobs = 101.headnode/0
     resources_available.mem = 100000mb
     resources_available.ncpus = 40
     resources_assigned.mem = 0kb
";
        let merged = merge_vnodes(parse_nodes(raw));
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].name, "host9"); // [n] dropped
        assert_eq!(merged[0].nproc, 80); // 40 + 40
        assert_eq!(merged[0].jobs.len(), 2); // jobs concatenated
        // 80 cores (>64) -> triple-line graph.
        let out = render("", raw, "");
        assert!(out.contains("host9") && !out.contains("host9["));
        assert!(out.contains('\u{22ee}') || out.contains('\u{2261}'));
    }

    #[test]
    fn render_node_graph_braille_midsize() {
        // A 48-core vnode (33-64) uses the Braille two-row graph; a fully-used column
        // is the centred ratio ∶, an empty one is =.
        let raw = "\
mid01[0]
     state = free
     jobs = 100.headnode/0, 101.headnode/1
     resources_available.mem = 100000mb
     resources_available.ncpus = 48
     resources_assigned.mem = 0kb
";
        let out = render("", raw, "");
        assert!(out.contains("mid01"));
        assert!(out.contains('\u{2236}')); // ∶ (a column with 2 cores used)
        assert!(out.contains('=')); // empty columns
        assert!(!out.contains('\u{22ee}') && !out.contains('\u{2261}')); // not triple
    }

    #[test]
    fn render_node_graph_quad_very_large() {
        // A 100-core node (>96) uses the four-per-column graph (⸬ used, ≣ free).
        let raw = "\
big01[0]
     state = free
     jobs = 100.headnode/0
     resources_available.mem = 100000mb
     resources_available.ncpus = 100
     resources_assigned.mem = 0kb
";
        let out = render("", raw, "");
        assert!(out.contains("big01"));
        assert!(out.contains('\u{2e2c}') && out.contains('\u{2263}')); // ⸬ and ≣
        assert!(!out.contains('\u{22ee}') && !out.contains('\u{2261}')); // not triple
    }

    #[test]
    fn render_gpu_column_only_when_present() {
        let out = render(JOBS, NODES, "bhar9988");
        // nodegpu02 has 2 gpus, 1 used -> "1/2" appears.
        assert!(out.contains("1/2"));
    }

    #[test]
    fn mem_helpers() {
        assert_eq!(parse_mem("2gb"), Some(2.0 * 1024.0 * 1024.0));
        assert_eq!(parse_mem("1024kb"), Some(1024.0));
        assert_eq!(parse_mem(""), None);
    }
}
