# monitor

A terminal UI for watching your PBS jobs and the cluster load, with a tabbed
view (System Load / Log Preview / Details), per-job log tailing, and resource
columns. It replaces the old `viddy`-based `monitor` and `tmux`-based `imonitor`
scripts and polls the headnode gently (see [Polling](#polling)).

## Requirements

- A Rust toolchain (`cargo`, `rustc`) — install via <https://rustup.rs> if absent.
- `ssh` access to `headnode` (PBS client commands run there; see [Polling](#polling)).
- The cluster tools `qstat` and `pbsnodes` available on `headnode`. The cluster-load
  and array-progress panels are rendered in-process (from `qstat`/`pbsnodes`); no
  external `qlload` or `qarray` script is needed.

## Build

```bash
cd ~/build/monitor
cargo build --release
```

The binary is produced at `~/build/monitor/target/release/monitor`.

## Install

The build produces three binaries: `monitor` (the TUI), `qlload` (standalone
cluster-load), and `qarray` (standalone array-progress). Symlink them into a
directory on your `PATH` (e.g. `~/.local/bin`), backing up any existing scripts
first, and replace `imonitor` with a shim so the old name still works:

```bash
BIN="$HOME/.local/bin"
REL="$HOME/build/monitor/target/release"
mkdir -p "$BIN"

# Back up existing scripts once (never clobber an existing backup).
for f in monitor imonitor qlload qarray; do
  if [ -e "$BIN/$f" ] && [ ! -e "$BIN/$f.bak" ]; then
    cp -a "$BIN/$f" "$BIN/$f.bak"
  fi
done

# Install: monitor + qlload + qarray -> release binaries; imonitor -> thin shim.
ln -sf "$REL/monitor" "$BIN/monitor"
ln -sf "$REL/qlload" "$BIN/qlload"
ln -sf "$REL/qarray" "$BIN/qarray"
printf '#!/bin/bash\nexec monitor "$@"\n' > "$BIN/imonitor"
chmod +x "$BIN/imonitor"
```

Confirm `~/.local/bin` is on your `PATH` (`echo "$PATH"`), then check:

```bash
monitor --help
```

Because `monitor` is a symlink to the build output, rebuilding
(`cargo build --release`) updates the installed command automatically — no need
to re-run the install steps.

## Usage

```
monitor [-u USERNAME] [--cluster-interval SECS] [--jobs-interval SECS]
```

- `-u` / `--username` — user to monitor and highlight (default: `$USER`). Switch
  live at any time with the `u` key.
- `--cluster-interval` — cluster-load poll interval, seconds (default: 30).
- `--jobs-interval` — your-jobs poll interval, seconds (default: 10).

### Keys

| Keys | Action |
|---|---|
| `←` / `→` | switch top tab (System Load / Log Preview / Details) |
| `↑`/`k`, `↓`/`j` | scroll the active tab by one line |
| `PgUp` / `PgDn` | scroll the active tab by one page |
| `Home`/`g`, `End`/`G` | jump to top / bottom of the active tab |
| `,` / `.` | previous / next running job (drives the Log Preview and Details tabs) |
| `a` / `q` / `r` | toggle the Array / Queued&Held / Running sections |
| `e` / `c` | toggle array-job compaction (collapse subjobs to `<num>[]`); both keys do the same |
| `u` | switch the monitored user (opens an empty text box; type a fragment and it fuzzy-matches cluster users, `→` shows the match; Enter confirms, Esc cancels) |
| `Ctrl-R` | refresh PBS data now |
| `Esc` / `Ctrl-C` | quit |

Sections appear only when they have data and auto-size to their content. Job ids
are shown without the `.headnode` suffix, and on a wide enough terminal the
Running and Queued & Held sections lay their rows out in as many columns as fit
(separated by a `┃` divider coloured to match the section, in a gap of at most 5
columns). When there
are more jobs than fit, the two sections split the space equally, each truncating
with a "N hidden" note, and the view auto-compacts array jobs to a single
`<num>[] ×N` row (toggle with `e`/`c`). Each tab opens scrolled to the end of its
content.

### Standalone `qlload`

The cluster-load panel is also its own command, printing the ANSI "User load" +
node-graph output once and exiting:

```
qlload [-u USERNAME]
```

- `-u` / `--username` — user to highlight (default: `$USER`).

It shares monitor's renderer and PBS fetch path, so it works from any node (over
the multiplexed SSH to `headnode`) with no external `qload`/Python dependency.

### Standalone `qarray`

The array-progress panel is likewise its own command, printing a progress bar per
array job (completed/running subjobs, plus an ETA from the mean subjob walltime):

```
qarray [-u USERNAME]
```

- `-u` / `--username` — user whose array jobs to show (default: `$USER`).

## Polling

The render loop never queries PBS; a background scheduler polls on its own
cadence and the UI draws from a cache. Your jobs come from a single
`qstat -u <user> -t -n1` per jobs-interval; cluster load from `qstat -ft` +
`pbsnodes -av` (rendered in-process, see `qlload.rs`) per cluster-interval; the
Details tab from `qstat -f <jobid>`, fetched once per job selection (rapid
switching coalesces to a single fetch); log preview is event-driven via inotify.
All PBS commands run on `headnode` over a multiplexed SSH connection (a
`Host headnode` ControlMaster block is added to `~/.ssh/config` on first run), so
they work from any compute node.

## Uninstall

Restore the original scripts from the backups:

```bash
BIN="$HOME/.local/bin"
[ -e "$BIN/monitor.bak" ] && mv "$BIN/monitor.bak" "$BIN/monitor"
[ -e "$BIN/imonitor.bak" ] && mv "$BIN/imonitor.bak" "$BIN/imonitor"
```
