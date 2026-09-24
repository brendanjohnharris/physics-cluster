---
name: code-physics
description: Use when developing code, running code, submitting PBS jobs, choosing a machine or queue, managing storage, or starting/debugging Kaimon REPL sessions on the USyd Physics cluster or shared HPCs (headnode, karl, orr, cartman, taiji, l40s/h100 GPUs, qsub, qstat, code tunnel, start_session timeouts, gate/hub failures).
---

# USyd Physics cluster

One PBS Pro cluster plus three scheduler-free shared machines, all mounting the same NFS filesystem. You are usually already on one of the shared machines (check `hostname`).

## Rule #1

**Never run heavy processes on headnode.** Headnode is only for routing (`qsub`, `qstat`, ssh hops), monitoring, and light editing. Everything computational runs on a compute node or a shared HPC.

## Topology

| Machine | Access | Cores | RAM | GPU | Notes |
|---|---|---|---|---|---|
| headnode | ssh | 24 | 124GB | – | PBS server; routing/monitoring ONLY |
| karl | ssh | 32 | 124GB | – | Shared HPC, no scheduler |
| cartman | ssh | 20 | 188GB | – | Shared HPC, no scheduler |
| orr | ssh | 8 | 62GB | Tesla M4 4GB | Shared HPC; GPU too small for real work |
| nodegpu01/02 | PBS | 64 | 247GB | 1 L40S each | queues: physics, l40s; 4 vnodes, GPU on `[3]` only (see GPU jobs) |
| h100g01 | PBS | 96 | 247GB | 1 H100 | queues: physics, h100; 4 vnodes, GPU on `[3]` only (see GPU jobs) |
| taiji01 | PBS | 32 | 247GB | – | queue: taiji (ACL) |
| cmt01/02, jasper01 | PBS | 168 | 1TB | – /1 | ACL queues; not ours |

## Filesystem (shared everywhere)

- `/suphys/<user>`: NFS home, **50GB quota, nearly full**. Configs/dotfiles only; never write data here.
- `/import/taiji1/bhar9988`: bulk storage (107TB volume, ~86% full). All code, data, job outputs, scratch. `~/code` symlinks here. Use DrWatson `datadir()` in projects.
- `/usr/physics`: shared software mount.

Because the filesystem is shared, the same Julia binary, project environments, and paths work on every machine; no copying needed.

## PBS essentials

Compute machines have **no PBS client**. Submit from anywhere via:

```bash
ssh headnode '/opt/pbs/bin/qsub /path/to/job.pbs'
ssh headnode 'qstat -u $USER'            # monitor
ssh headnode 'qdel <jobid>'              # kill (own jobs only)
```

**Always specify `mem`, `walltime`, and `vmem=mem`**: queue defaults are 1GB and 1–2h, which silently kill under-specified jobs.

### Logging and detecting when a job has finished

`qsub` returns as soon as the job is *queued*, so its exit status says nothing about the work.

**Write the log to `~/.jobs/${PBS_JOBID}.log` through `tee`, not via `#PBS -o`.** `#PBS -o` is *spooled*:
the file is copied into place only when the job ENDS, so while a job runs that path still holds the
**previous** run's output. Grepping it for `exit=` or `ERROR` then matches stale text and reports a fresh
job as already finished. `tee` writes in real time, so the log is a genuine progress signal (this is what
`~/code-physics/code-physics.pbs` does).

```bash
#PBS -N myjob
#PBS -l select=1:ncpus=32:mem=100GB:vmem=100GB
#PBS -l walltime=04:00:00
set -o pipefail                    # so $? is the work's exit status, not tee's
mkdir -p "$HOME/.jobs"
{
  echo "host=$(hostname) cores=$(nproc) start=$(date -Is)"
  <the work>
  echo "exit=$? end=$(date -Is)"
} 2>&1 | tee -a "$HOME/.jobs/${PBS_JOBID}.log"
```

Then watch it live, and poll `qstat` by **jobid** (not job name) for the definitive state:

```bash
tail -f ~/.jobs/23767.headnode.log
until ! ssh headnode 'qstat -u $USER' 2>/dev/null | grep -q '^23767'; do sleep 60; done
ssh headnode 'qstat -xu $USER'     # -x also lists already-finished jobs
```

**Three remaining traps.**

1. **Julia block-buffers stdout/stderr when it is not a TTY**, so even a `tee`d log can sit empty and
   then flush everything at exit. Put `flush(stdout)` after each progress print in the Julia script;
   an empty log is otherwise no evidence of a hung job — check `qstat` or the process.
2. **A job absent from `qstat` has *finished*, not necessarily *succeeded*.** Read the log tail for the
   `exit=` marker before believing it worked.
3. For long sweeps, also have the script checkpoint its **results** incrementally (a `.tsv` rewritten
   each iteration). Row count is then a progress bar, and a kill costs one iteration.

**`/tmp` is node-local.** A script or input under `/tmp` on the submitting machine does not exist on the
compute node, and the job dies instantly with `No such file or directory`. Everything the job touches —
script, data, output — must live on the shared filesystem (`/import/taiji1`; `~/.jobs` is fine for logs,
they are small, but never bulk data since `/suphys` is nearly at quota).

| Queue | Route/limits | Use |
|---|---|---|
| defaultQ | → physics; max 48 cpus/job and /user, no GPUs | default CPU work |
| taiji | ACL (we have access); taiji01 | second default |
| l40s | ≤48 cpus, ≤2 GPUs, ≤72h | L40S GPU jobs |
| h100 | ≤48 cpus, 1 GPU, ≤72h | H100 GPU jobs |
| cmt, jasper* | ACL; not ours | don't target |

## Where to run: decision ladder

For quick prototyping and small tests (seconds–minutes), take the first rung that fits:

1. **Kaimon REPL** connected to the current project, or startable for it (see below).
2. **Local script on the current machine**, if it is under ~75% load/memory (`cat /proc/loadavg; free -g`).
3. **qsub to defaultQ** via headnode, if the cluster has free capacity (`ssh headnode pbsnodes -aSj`).
4. **ssh to another shared HPC** (karl/cartman/orr) that is under 75% occupied.
5. Otherwise run locally and wait.

### Kaimon's allowed-projects list

`start_session` only spawns a REPL for a project on Kaimon's allow list
(`~/.config/kaimon/projects.json`, also editable in the TUI Config tab). For a project
that is not on it the call does not fail fast; it waits on an authorisation nobody
answers and returns "No response to the approval prompt within 50s". That message invites
a re-try, so it is easy to burn several minutes and conclude that REPLs need permission.
They do not: spawning is agent-initiated, and Claude Code's own permissions are not
involved (check `~/.claude/settings.json` if in doubt).

Diagnose in two calls: `start_session` with no arguments lists the allowed projects and
their status, and `server_log` shows whether the request reached the server at all (a
non-whitelisted project leaves no entry).

**Adding the project yourself is fine, and is the fix.** Append an entry and call
`start_session` again:

```json
{ "enabled": true, "project_path": "/import/taiji1/bhar9988/code/<Project>" }
```

Use the absolute path of the directory holding the `Project.toml` that the REPL should
activate, which for a sub-environment (`<Project>/_research`) is that subdirectory, not
the repo root. Keep the existing entries, and mention the addition in your reply.

With more than one session connected, `ex` needs to be told which to use, and the
parameter is **`ses`** (the 8-character key), not `session`. Passing `session` arrives
empty and returns `No session matched ''`, which reads like a dead session rather than a
misnamed argument.

### Allowed, but the session still never connects

There is a second failure that looks identical from the tool side and has nothing to do
with the allow list. `server_log` separates them in one call:

- **Not allowlisted** — no entry at all, and the message mentions the approval prompt.
- **Allowlisted, registration broken** — `Session '<name>' subprocess spawning (PID=…)`
  followed exactly 120 s later by `Session '<name>' startup timed out`.

In the second case the REPL is fine. Its log
(`~/.cache/kaimon/sessions/<project>.log`) reaches `Kaimon gate connected`, and the gate
writes its discovery metadata, but the connection never lands in the server's manager, so
`ping` reports `0 connected` while the gate believes otherwise. The `mode=:ipc` discovery
path is the broken one.

**Do not retry.** `session_manager.jl:452` only sets `status = :crashed` on timeout; it
never kills the subprocess, so every attempt leaks a **~600 MB orphaned Julia process**.
Compare `ps -eo pid,rss,cmd | grep KaimonGate` against `ping`'s session list and reap the
difference. Restarting the hub does **not** fix this — verified against a fresh hub.

Bypass ipc discovery with an explicit TCP gate:

```bash
setsid nohup julia -t auto --project=. --startup-file=no -e '
using KaimonGate
@async KaimonGate.serve(mode=:tcp, port=9876, force=true, allow_mirror=true, spawned_by="agent")
while true; sleep(3600); end' >/dev/null 2>&1 </dev/null &
```

then `connect_tcp(host="127.0.0.1", port=9876)`. `$!` is `setsid`'s pid, not Julia's, so
wait on the port (`ss -tln | grep 9876`) rather than on that pid. The gate activates the
project env, so anything needing test-only deps (CairoMakie, say) still wants a separate
environment.

Note also that the `start_session` MCP call gives up **sooner** than Kaimon's own 120 s
budget, so a slow-but-successful start still reports a timeout. Call `ping` a minute later
instead of re-issuing `start_session`.

### When the hub itself is wedged

Distinct again: `:2828` stops answering but nothing dies. `kaimon-health` gives a verdict,
and `kaimon-restart` (both in `~/.local/bin`) kills the hub on whichever node advertised it
and claims a fresh one, reusing the federation token. It drops every connected gate, so
check what you would lose first. It does not help with the registration failure above.


## Large jobs (hours+, many cores)

Choose by the script's needs:

- **Small self-contained Julia scripts**: raw `qsub` via ssh headnode is fine.
- **Anything that benefits from tooling** (logging, heap hints, project activation, array jobs): use [AcademicClusters.jl](https://github.com/brendanjohnharris/AcademicClusters.jl) `USydPhysics`:
  - `runscript("script.jl"; ncpus, mem, walltime, queue)` → single job, returns `(jobid, logfile)`.
  - `runscripts(exprs_or_dir; ...)` → PBS array job over numbered scripts.
  - `addprocs(n; ncpus, mem, walltime, queue)` → Distributed.jl workers as PBS jobs.
  - `selfdestruct()` → qdel the current job from inside it.
- **Embarrassingly parallel sweeps**: `distributeprocs(np; ncpus, mem, walltime)` spreads workers across defaultQ + taiji **and** karl/cartman/orr by live free capacity (`np = Inf` fills everything, minus a buffer; `saturation = 0.75` keeps shared HPCs polite).

Default queues: **defaultQ and taiji**.

`taiji` runs only on taiji01 (one node), so arrays split across both queues can strand their `taiji` half when
taiji01's memory is full (comment `Insufficient amount of resource: Qlist`) while `physics` has room. Check
`pbsnodes -aSj` and move still-queued arrays with `qmove defaultQ <id>[]` (`physics` is route-only; an array
with running subjobs cannot move). Other queues: `cmt`/`jasper*` are ACL'd (not us); `l40s`/`h100` need ≥1 GPU.

All jobs submitted directly with qsub should follow the structure of ~/submit; crucially, the line `bash $file 2>&1 | tee /suphys/bhar9988/.jobs/${PBS_JOBID}.log` ensures that logs are written to the standardized `~/.log` directory

## GPU jobs

1. First check for a local GPU (`nvidia-smi`), you might be running on a gpu node but the shared HPCs effectively have none, so then GPU work goes through PBS.
2. Pick l40s or h100 by workload; no confirmation needed, but respect queue GPU limits (h100: 1 GPU; l40s: ≤2) and the 72h walltime cap.

### GPU jobs get ~62 GB of host memory, in practice

Each GPU node is split into four vnodes of ~62 GB (16 CPUs on nodegpu01/02, 24 on h100g01). The GPU
sits on `[3]`, whose `Qlist` is only `l40s`/`h100`; `[0]`–`[2]` are `Qlist = physics,<gpu queue>`. A chunk
may span vnodes of one host, so on an idle node a GPU job could take up to ~248 GB, and the queues set no
`resources_max.mem`. But `[0]`–`[2]` are normally `job-busy` (every CPU held by `physics` array jobs),
and the scheduler places nothing on a `job-busy` vnode, not even a memory-only (`ncpus=0`) chunk. So a GPU
job over ~60 GB sits queued indefinitely (verified 2026-09-23 with 100 GB as one chunk, as a packed
`GPU+memory` two-chunk select, and as job-wide `-l ngpus=1 -l mem=100GB`, which PBS rewrites to the same
select). The comment reads `Insufficient amount of resource: Qlist`, which is misleading: `Qlist` is just
the host-level resource the queue stamps on every chunk; the shortfall is memory on non-busy vnodes.

Check before submitting: `pbsnodes -av` per-vnode `state`, `resources_available.mem` and
`resources_assigned.mem`. Keep GPU jobs under ~60 GB host memory, or ask the admins. Try
`julia --heap-size-hint=45G` first: Julia's GC otherwise lets garbage (e.g. per-window device→host copies)
pile up past the cgroup limit. That alone took WorkingRegime's `demo_run.jl` from killed at 66 GB to a
49 GiB peak in a 58 GB `l40s` job, with no code change.

## Autonomy limits

- Submit freely up to **32 cpus, 100GB, 4h**; confirm with the user for anything larger.
- Never `qdel` jobs you didn't start; in particular the **code-physics VSCode tunnel job** (`qstat -u $USER | grep code-phys`) is the user's session, hands off.
- One code tunnel exists at a time; it's started with `~/code-physics/start-code-physics`, logs to `~/code-physics/code-physics.log`.

## Common mistakes

- Running computation on headnode (rule #1).
- Omitting `mem`/`walltime` on qsub: job dies at 1GB/1–2h defaults.
- Writing large outputs to `/suphys` home: quota is nearly exhausted; use `/import/taiji1`.
- Trying `qsub` on karl/orr/cartman directly: no PBS client; go via `ssh headnode`.
- Saturating a shared HPC: stay under 75% of cores and available memory.
- Assuming orr's Tesla M4 is usable for ML: 4GB, ancient; use l40s/h100 queues.
