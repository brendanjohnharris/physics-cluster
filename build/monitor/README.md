# monitor

A terminal UI for watching your PBS jobs and the cluster load, with a tabbed
view (System Load / Log Preview / Details), per-job log tailing, and resource
columns. It replaces the old `viddy`-based `monitor` and `tmux`-based `imonitor`
scripts and polls the headnode gently (see [Polling](#polling)).

## Requirements

- A Rust toolchain (`cargo`, `rustc`) — install via <https://rustup.rs> if absent.
- `ssh` access to `headnode` (PBS client commands run there; see [Polling](#polling)).
- The cluster tools `qstat`, `qarray`, and `qlload` available on `headnode`.

## Build

```bash
cd ~/build/monitor
cargo build --release
```

The binary is produced at `~/build/monitor/target/release/monitor`.

## Install

Symlink the binary into a directory on your `PATH` (e.g. `~/.local/bin`), backing
up any existing `monitor`/`imonitor` first, and replace `imonitor` with a shim so
the old name still works:

```bash
BIN="$HOME/.local/bin"
REL="$HOME/build/monitor/target/release/monitor"
mkdir -p "$BIN"

# Back up existing scripts once (never clobber an existing backup).
for f in monitor imonitor; do
  if [ -e "$BIN/$f" ] && [ ! -e "$BIN/$f.bak" ]; then
    cp -a "$BIN/$f" "$BIN/$f.bak"
  fi
done

# Install: monitor -> release binary; imonitor -> thin shim.
ln -sf "$REL" "$BIN/monitor"
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
monitor [USERNAME] [--cluster-interval SECS] [--jobs-interval SECS]
```

- `USERNAME` — whose jobs to show (default: `$USER`).
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
| `a` / `q` / `u` | toggle the Array / Queued&Held / Running sections |
| `r` | refresh PBS data now |
| `Esc` / `Ctrl-C` | quit |

Sections appear only when they have data and auto-size to their content. Each
tab opens scrolled to the end of its content.

## Polling

The render loop never queries PBS; a background scheduler polls on its own
cadence and the UI draws from a cache. Your jobs come from a single
`qstat -u <user> -t -n1` per jobs-interval; cluster load from `qlload` per
cluster-interval; the Details tab from `qstat -f <jobid>`, fetched once per job
selection (rapid switching coalesces to a single fetch); log preview is
event-driven via inotify. `qstat`/`qarray`/`qstat -f` run on `headnode` over a
multiplexed SSH connection (a `Host headnode` ControlMaster block is added to
`~/.ssh/config` on first run), so they work from any compute node.

## Uninstall

Restore the original scripts from the backups:

```bash
BIN="$HOME/.local/bin"
[ -e "$BIN/monitor.bak" ] && mv "$BIN/monitor.bak" "$BIN/monitor"
[ -e "$BIN/imonitor.bak" ] && mv "$BIN/imonitor.bak" "$BIN/imonitor"
```
