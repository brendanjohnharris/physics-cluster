#! /bin/bash

# * Fix malformed 'ml' function exported by Environment Modules
# The login environment exports BASH_FUNC_ml%% missing its closing brace, so
# every new shell prints "error importing function definition for 'ml'". The
# variable name contains '%%' and cannot be removed with `unset`, so strip it
# with `env -u` and re-exec this interactive shell once. The exported sentinel
# guards against an infinite re-exec loop. Kept first so the rest of this file
# (Kaimon hub-claim, SSH tunnels, etc.) runs only once, in the cleaned shell.
if [[ $- == *i* && -z ${_ML_SCRUBBED:-} ]] \
   && grep -qaz 'BASH_FUNC_ml%%=' /proc/self/environ 2>/dev/null; then
    export _ML_SCRUBBED=1
    _ml_flags=(); shopt -q login_shell && _ml_flags=(-l)
    exec env -u 'BASH_FUNC_ml%%' "${BASH:-/bin/bash}" "${_ml_flags[@]}"
fi

# * Source global definitions
if [ -f /etc/bashrc ]; then
    . /etc/bashrc
fi
if [ -f "$HOME/.shellfishrc" ]; then
    . "$HOME/.shellfishrc"
fi

# * Environment variables
export LD_LIBRARY_PATH=""
export LD_PRELOAD=""
export VSCODE_CLI_USE_FILE_KEYCHAIN=1
export WORKDIR="/import/taiji1/bhar9988"
export TERM="xterm-256color"
export PATH="$HOME/.local/bin/:$PATH"
export PATH="/usr/physics/pbspro/bin:$PATH"
export PAGER="vim -R +AnsiEsc"

# * Save space
export XDG_CACHE_HOME=$WORKDIR/.cache
export UV_CACHE_DIR=$WORKDIR/.cache/uv
export RUSTUP_HOME=$WORKDIR/.rustup
export PIXI_CACHE_DIR=$WORKDIR/.cache/rattler/cache
export RATTLER_CACHE_DIR=$WORKDIR/.cache/rattler/cache

# * Julia configuration
export JULIA_CPU_TARGET="generic;icelake-client,clone_all;haswell,clone_all;broadwell,clone_all;sandybridge,clone_all;ivybridge,clone_all;znver3,clone_all;sapphirerapids,clone_all"
export JULIA_PKG_USE_CLI_GIT=true
export LD_LIBRARY_PATH=""
export FREETYPE_ABSTRACTION_FONT_PATH="/suphys/bhar9988/.pixi-default/.pixi/envs/default/fonts/"
export PATH="/suphys/bhar9988/build/miniforge3/envs/bhar9988/bin:$PATH"
export PATH="/suphys/bhar9988/.julia/bin/:$PATH"
export PYTHON="/suphys/bhar9988/build/miniforge3/envs/bhar9988/bin/python"
# export JULIA_DEBUG="SpatiotemporalMotifs" # loading,VSCodeServer
export JULIA_WORKER_TIMEOUT=600
export USYDCLUSTERS_LOGDIR="/suphys/bhar9988/.jobs/"
export DRWATSON_STOREPATCH=true
export JULIA_NUM_THREADS="auto"
# export JULIA_CONDAPKG_OFFLINE="yes"
# export ALLEN_NEUROPIXELS_OFFLINE="true"
export JULIA_HISTORY="$HOME/.julia/logs/repl_history.jl"
export JULIA_COPY_STACKS=1
export GIT_SSL_CAINFO=/etc/pki/ca-trust/extracted/pem/tls-ca-bundle.pem
export SSL_CERT_FILE=/etc/pki/ca-trust/extracted/pem/tls-ca-bundle.pem
export CURL_CA_BUNDLE=/etc/pki/ca-trust/extracted/pem/tls-ca-bundle.pem




# export PYTHONWARNINGS="ignore::ImportWarning,ignore::UserWarning,ignore::DeprecationWarning"
# export PYTHONWARNINGS="ignore:JuliaCompatHooks.find_spec() not found:ImportWarning,ignore:pkg_resources is deprecated as an API:UserWarning"
# export JULIA_PYTHONCALL_EXE="@PyCall"
# export FATHOM_PATCHES= true
# export DEWDROP_BACKEND= "gpu"
# export XLA_PYTHON_CLIENT_PREALLOCATE= false # For multiple JAX instances

if [[ $(hostname) =~ headnode* ]]; then
    export JULIA_NUM_THREADS="1"
    export SM_CLUSTER=true
fi
if [[ -n "$PBS_JOBID" ]]; then
    export SM_CLUSTER=true
    if [[ -z "${MOST_RECENT_SOCKET}" ]]; then
        # MOST_RECENT_SOCKET is either unset or empty
        : # Do nothing, similar to the original's "else" block when empty
    else
        # MOST_RECENT_SOCKET is set and not empty
        tunnelrecentsocket
    fi
fi

# * System configuration
ulimit -c 0
umask 022

# * Aliases
alias qdell='qselect -u $USER | xargs qdel'
# alias qlload='set ncols=`tput cols`; env COLUMNS=200 qload | fold -c"$ncols"'
alias code-physics='$HOME/code-physics/start-code-physics'
alias code-physics-2='$HOME/code-physics/start-code-physics --workdir $HOME/code-physics/cli2/ --name code-physics-2'
config() {
    git --git-dir="$HOME/physics-cluster" --work-tree="$HOME" "$@"
}

# * Common modules
# module load pbspro
# module load gsl-2.7
# module load hdf/5/1.14.1-2_intel2021





export NVM_DIR="$HOME/.nvm"
[ -s "$NVM_DIR/nvm.sh" ] && \. "$NVM_DIR/nvm.sh"  # This loads nvm
# [ -s "$NVM_DIR/bash_completion" ] && \. "$NVM_DIR/bash_completion"  # This loads nvm bash_completion
. "$HOME/.cargo/env"

export PATH="/suphys/bhar9988/.pixi/bin:$PATH"

# * Kaimon --- one hub (headless MCP server) per user, federated over SSH
# Nodes share this NFS $HOME, so a single hub on one node serves REPLs on every
# node. The FIRST interactive login claims the hub: it starts the :2828 headless
# server and records its location + auth token in ~/.cache/kaimon/hub.json (a
# metadata signpost --- never a socket; live sockets on shared $HOME cross-wire
# between hosts). Later logins on other nodes see a live hub and do nothing here;
# their Julia REPL (startup.jl) starts a :tcp gate, tunnels it to the hub over
# SSH, and registers in ~/.config/kaimon/tcp_gates.json, which the hub auto-polls.
#
# Liveness is a TCP probe of the advertised host:port --- valid across nodes,
# unlike a PID check. headnode is excluded entirely (guard below); startup.jl
# additionally refuses to register against a hub whose host is headnode.
#
# headnode must carry no long-lived processes, so the hub is never started,
# claimed, or linked there. Detect it via BOTH the short hostname and the FQDN
# (hub.json records `hostname -f`, which can differ from `hostname`); a single
# helper keeps every Kaimon guard in lockstep.
_kaimon_on_headnode() {
    case "$(hostname)"    in *headnode*) return 0;; esac
    case "$(hostname -f)" in *headnode*) return 0;; esac
    return 1
}
if [[ $- == *i* ]] && ! _kaimon_on_headnode; then
    # Kaimon derives its cache (sockets, state) from XDG_CACHE_HOME; this .bashrc
    # redirects XDG_CACHE_HOME (see above), so honor it here too or hub.json and
    # Kaimon's runtime state would land in different directories.
    _kcache="${XDG_CACHE_HOME:-$HOME/.cache}/kaimon"
    mkdir -p "$_kcache"
    _khub="$_kcache/hub.json"
    # Probe the advertised hub (host:port from hub.json) for liveness.
    _khub_live() {
        [ -f "$_khub" ] || return 1
        local h p
        h=$(sed -n 's/.*"host"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' "$_khub" | head -1)
        p=$(sed -n 's/.*"mcp_port"[[:space:]]*:[[:space:]]*\([0-9]*\).*/\1/p' "$_khub" | head -1)
        [ -n "$h" ] && [ -n "$p" ] || return 1
        (exec 3<>"/dev/tcp/$h/$p") 2>/dev/null || return 1
        exec 3<&-; return 0
    }
    (
        flock -n 9 || exit 0
        if ! _khub_live; then
            # No live hub --- claim it on this node.
            nohup "$HOME/.julia/bin/kaimon" --headless \
                >"$_kcache/headless.log" 2>&1 &
            # Wait for :2828 to bind (cold start loads packages, ~15-20s).
            for _ in $(seq 1 45); do
                (exec 3<>/dev/tcp/127.0.0.1/2828) 2>/dev/null && { exec 3<&-; break; }
                sleep 1
            done
            # Reuse the existing token if present, else mint one.
            _ktok=$(sed -n 's/.*"token"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' "$_khub" 2>/dev/null | head -1)
            [ -n "$_ktok" ] || _ktok=$( (command -v uuidgen >/dev/null && uuidgen) || cat /proc/sys/kernel/random/uuid)
            printf '{\n  "host": "%s",\n  "mcp_port": 2828,\n  "token": "%s",\n  "pid": %s,\n  "started_at": "%s"\n}\n' \
                "$(hostname -f)" "$_ktok" "$$" "$(date -Is)" > "$_khub"
            echo -e "\e[34mKaimon: claimed hub on $(hostname -f) (:2828)\e[0m"
        else
            _khh=$(sed -n 's/.*"host"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' "$_khub" | head -1)
            echo -e "\e[34mKaimon: hub already live on ${_khh}; this REPL will register as a gate\e[0m"
        fi
    ) 9>"$_kcache/.headless.flock"
    unset -f _khub_live 2>/dev/null; unset _khub _kcache
fi

# * Kaimon MCP link --- forward localhost:2828 to the hub for local MCP clients.
# The hub binds 127.0.0.1:2828 on its OWN node and IP-allowlists 127.0.0.1/::1, so
# an MCP client (e.g. Claude) on any other node cannot reach it directly. An SSH
# local-forward fixes both layers at once: the request egresses on the hub's
# loopback, so it hits the loopback-bound server AND presents as 127.0.0.1. This
# mirrors the gate federation, which already tunnels hub<->gate over SSH.
#   kaimon-link        # ensure localhost:2828 reaches the current hub (verbose)
#   kaimon-link quiet  # same, silent --- used by the login auto-run below
# Pins to whichever node hub.json names; re-run after a hub re-election/migration.
kaimon-link() {
    local hj="${XDG_CACHE_HOME:-$HOME/.cache}/kaimon/hub.json" h p
    [ -f "$hj" ] || { [ "$1" = quiet ] || echo "kaimon-link: no hub.json"; return 1; }
    h=$(sed -n 's/.*"host"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' "$hj" | head -1)
    p=$(sed -n 's/.*"mcp_port"[[:space:]]*:[[:space:]]*\([0-9]*\).*/\1/p' "$hj" | head -1); : "${p:=2828}"
    # Already reachable on loopback? (we're the hub, or a tunnel is up.)
    if timeout 2 bash -c "(exec 3<>/dev/tcp/127.0.0.1/$p) 2>/dev/null"; then
        [ "$1" = quiet ] || echo "kaimon-link: localhost:$p already live"; return 0
    fi
    if [ "$h" = "$(hostname -f)" ]; then
        [ "$1" = quiet ] || echo "kaimon-link: this IS the hub but :$p not bound --- run 'kaimon --headless'"; return 1
    fi
    [ "$1" = quiet ] || echo "kaimon-link: tunnelling localhost:$p -> $h:$p"
    ssh ${KAIMON_SSH_OPTS:-} -N -f -L "$p:127.0.0.1:$p" "$h" 2>/dev/null
}

# Auto-link on interactive login (best-effort, never blocks the shell: BatchMode
# fails fast if node->node SSH needs a prompt/ticket; backgrounded + disowned).
# No-op on the hub node (port already live) and on headnode.
if [[ $- == *i* ]] && ! _kaimon_on_headnode; then
    KAIMON_SSH_OPTS="-o BatchMode=yes -o ConnectTimeout=5" kaimon-link quiet & disown 2>/dev/null
fi
unset -f _kaimon_on_headnode 2>/dev/null

# >>> juliaup initialize >>>

# !! Contents within this block are managed by juliaup !!

case ":$PATH:" in
    *:/suphys/bhar9988/.juliaup/bin:*)
        ;;

    *)
        export PATH=/suphys/bhar9988/.juliaup/bin${PATH:+:${PATH}}
        ;;
esac
# Tab completion for juliaup and julia channel selection
[ -f "/suphys/bhar9988/.julia/juliaup/completions/bash.sh" ] && source "/suphys/bhar9988/.julia/juliaup/completions/bash.sh"

# <<< juliaup initialize <<<
