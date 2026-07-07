ENV["JULIA_CPU_TARGET"] = "generic;icelake-client,clone_all;haswell,clone_all;broadwell,clone_all;sandybridge,clone_all;znver3,clone_all;sapphirerapids,clone_all;graniterapids,clone_all"
ENV["JULIA_PKG_USE_CLI_GIT"] = true
ENV["PATH"] = "/import/taiji1/bhar9988/.conda/envs/bhar9988/bin:$(ENV["PATH"])"
using Pkg
ENV["PYTHON"] = "/import/taiji1/bhar9988/.conda/envs/bhar9988/bin/python"
# ENV["JULIA_PYTHONCALL_EXE"] = "@PyCall"
ENV["JULIA_DEBUG"] = "SpatiotemporalMotifs" # loading,VSCodeServer
ENV["JULIA_DISTRIBUTED"] = true
ENV["JULIA_WORKER_TIMEOUT"] = 600
ENV["DRWATSON_STOREPATCH"] = true
# ENV["FORESIGHT_PATCHES"] = true
# ENV["DEWDROP_BACKEND"] = "gpu"
# ENV["XLA_PYTHON_CLIENT_PREALLOCATE"] = false # For multiple JAX instances
# ENV["XLA_PYTHON_CLIENT_ALLOCATOR"] = "platform" # Deallocates??
# ENV["XLA_PYTHON_CLIENT_PREALLOCATE"] = "false"
ENV["XLA_PYTHON_CLIENT_ALLOCATOR"] = "platform"

# NB: the global logger is deliberately left as the stdlib default. A custom TerminalLogger in the
# global slot crashes GPU kernel compilation (GPUCompiler introspects the global logger's
# min_enabled_level at a fixed world → "method too new"). Scripts that want a progress bar scope it
# locally instead: `with_logger(TerminalLogger()) do ... end` (world-safe; see scripts/plots/critical_demo.jl).

using Revise
using OhMyREPL
using Downloads
using Infiltrator
colorscheme!("OneDark")
enable_autocomplete_brackets(false)
function template()
    return @eval begin
        using PkgTemplates
        Template(;
            user = "brendanjohnharris",
            dir = "./",
            julia = v"1.10.0",
            plugins = [
                ProjectFile(),
                SrcDir(),
                Tests(; project = true),
                Readme(),
                License(),
                Git(; ignore = ["*.code-workspace", "*.mat", "*.csv", "*.parquet", "*.jld2", "data", "*.jl.cov", "*.jl.*.cov", "*.jl.mem", "docs/build/", "docs/site/", "LocalPreferences.toml", ".CondaPkg/", "Artifacts.toml", "Manifest.toml", ".vscode"]),
                CompatHelper(),
                TagBot(),
                GitHubActions(; linux = true, osx = true, windows = true, x86 = true, extra_versions = ["1.10", "1.11", "1.12", "pre"]),
                Codecov(),
                Documenter{GitHubActions}(),
                Dependabot(),
                RegisterAction(),
                Runic(),
            ],
        )
    end
end

#run(`conda activate $(joinpath(Base.active_project(), "../.CondaPkg/env/"))`)
if !contains(gethostname(), "headnode") && haskey(ENV, "MOST_RECENT_SOCKET")
    sock = ENV["MOST_RECENT_SOCKET"]

    # cmd = Pipeline(`/bin/bash`, `ssh -nNT -L $sock:$sock headnode \&`)
    # if !issocket(sock)
    #     run(cmd)
    # end
    # using Sockets
    # using Dates

    # pushfirst!(LOAD_PATH, raw"/taiji1/bhar9988/.vscode-server/extensions/julialang.language-julia-1.72.0/scripts/packages")
    # try
    #     using VSCodeServer
    # finally
    #     popfirst!(LOAD_PATH)
    # end
    # VSCodeServer.serve(sock; is_dev="DEBUG_MODE=true" in Base.ARGS, crashreporting_pipename=raw"/tmp/vsc-jl-cr")
    # nothing # re-establishing connection with VSCode
end
# if contains(gethostname(), "gpu") || contains(gethostname(), "h100")
#     using CUDA
#     if CUDA.runtime_version() != v"12.5"
#         CUDA.set_runtime_version!(v"12.5.0")
#     end
# end


function clean_repl_history(
        path = get(
            ENV, "JULIA_HISTORY",
            joinpath(homedir(), ".julia", "logs", "repl_history.jl")
        )
    )
    return try
        isfile(path) || return
        bytes = read(path)
        bytes = filter(!=(0x00), bytes)                 # remove NULs
        lines = split(String(bytes), "\n")
        lines = filter(l -> !all(isspace, l), lines)    # remove blank/whitespace
        lines = map(lines) do l                          # ensure code lines are tab-indented
            (startswith(l, '\t') || startswith(l, '#')) ? l : "\t" * l
        end
        write(path, join(lines, "\n") * "\n")
    catch err
        @warn "Failed to clean REPL history" exception = (err, catch_backtrace())
    end
end

_safe_clean_repl_history() =
try
    clean_repl_history()
catch err
    @warn "Failed to clean REPL history (wrapper)" exception = (err, catch_backtrace())
end

@async _safe_clean_repl_history()  # clean soon after startup
atexit(_safe_clean_repl_history)   # clean again on normal exit

# Kaimon Gate — connect this REPL to the federated hub over TCP.
#
# Nodes share one NFS $HOME, so Kaimon's default IPC mode (Unix sockets under
# ~/.cache/kaimon/sock) cross-wires between hosts: results mirror locally but the
# server's PUB stream binds a stale/foreign socket, so eval completions never
# return. We instead run the gate in :tcp mode bound to 127.0.0.1, and bridge it
# to the hub (recorded in ~/.cache/kaimon/hub.json by .bashrc) over SSH.
#
# Roles, decided by comparing this node's FQDN to hub.json's host:
#   • HUB node    — gate binds 127.0.0.1; hub connects via loopback (no tunnel).
#   • REMOTE node — open a reverse SSH tunnel mapping our gate's REP+PUB ports
#     onto the hub's loopback at a per-node port pair, then register that pair in
#     tcp_gates.json. The hub's poll loop (_poll_tcp_gates!) connects us.
# Either way we add our entry to ~/.config/kaimon/tcp_gates.json (shared NFS,
# metadata only) — idempotent; the hub keys connections by host:port.
try
    using Revise
catch e
    @info "ℹ Revise not loaded (optional - install with: Pkg.add(\"Revise\"))"
end

module _KaimonFed
    # Self-contained helpers. Deliberately NO package deps (JSON isn't in the global
    # env startup.jl runs in): hub.json is flat, so we read it with regexes and write
    # tcp_gates.json by hand. The hub re-parses tcp_gates.json with a real JSON
    # parser, so we only need to emit valid, minimal JSON.

    # Match Kaimon's own dir resolution: cache honors XDG_CACHE_HOME (this user
    # redirects it via .bashrc); config dir is plain ~/.config/kaimon (not redirected).
    const CACHE_DIR = get(ENV, "XDG_CACHE_HOME", joinpath(homedir(), ".cache"))
    const HUB_JSON = joinpath(CACHE_DIR, "kaimon", "hub.json")
    const GATES_JSON = joinpath(homedir(), ".config", "kaimon", "tcp_gates.json")
    # Gate REP/PUB ports are NOT fixed: each REPL binds ephemeral loopback ports
    # (Gate.serve(port=0)) so multiple REPLs can run on one node without colliding
    # on a shared port. The actually-bound ports are read back from Gate._TCP_PORT[]
    # / _TCP_STREAM_PORT[] after serve and registered in tcp_gates.json.

    # Pull a single "key": "value" string field out of flat hub.json.
    function hub_field(s::AbstractString, key::AbstractString)
        m = match(Regex("\"" * key * "\"\\s*:\\s*\"([^\"]*)\""), s)
        return m === nothing ? "" : String(m.captures[1])
    end
    read_hub() = isfile(HUB_JSON) ? read(HUB_JSON, String) : nothing

    # Deterministic per-node hub-facing port pair (even=REP, odd=PUB) in 19000–19998,
    # so distinct nodes never collide on the hub's loopback and re-registration is
    # idempotent (same host → same pair → same "tcp-127.0.0.1-<H>" session id).
    function hub_port_pair(host::AbstractString)
        h = UInt32(0x811c9dc5)
        for b in codeunits(host)            # FNV-1a; UInt32 wraps automatically
            h = (h ⊻ b) * UInt32(0x01000193)
        end
        base = 19000 + 2 * Int(h % 0x01f4)   # %500 → 19000,19002,…,19998
        return (base, base + 1)
    end

    # Build a tcp_gates.json line for one entry. `node`/`pid` identify the
    # registering REPL so this node can later prune its own dead entries (see
    # upsert_gate!); the hub ignores these extra fields when parsing.
    _gate_json(; host, port, stream_port, token, name, node, pid) = string(
        "    {\n",
        "      \"host\": \"", host, "\",\n",
        "      \"port\": ", port, ",\n",
        "      \"name\": \"", name, "\",\n",
        "      \"node\": \"", node, "\",\n",
        "      \"pid\": ", pid, ",\n",
        "      \"enabled\": true,\n",
        "      \"token\": \"", token, "\",\n",
        "      \"stream_port\": ", stream_port, "\n",
        "    }"
    )

    # Upsert one entry into tcp_gates.json (shared NFS). We can't depend on JSON, so
    # we preserve any *other* nodes' entries by keeping every existing object block
    # whose host:port differs from ours, then re-emit the file. Each entry is a
    # brace-delimited block inside "tcp_gates": [ ... ].
    function upsert_gate!(; host, port, stream_port, token, name, node, pid)
        mkpath(dirname(GATES_JSON))
        existing = String[]
        if isfile(GATES_JSON)
            txt = read(GATES_JSON, String)
            # Match only *gate* objects: brace blocks containing a "host" key. This
            # excludes the envelope object {"tcp_gates":[...]}, which has no "host".
            for m in eachmatch(r"\{[^{}]*\"host\"[^{}]*\}", txt)
                blk = m.match
                h = hub_field(blk, "host")
                p = (mm = match(r"\"port\"\s*:\s*(\d+)", blk); mm === nothing ? -1 : parse(Int, mm.captures[1]))
                (h == host && p == port) && continue       # drop our own prior entry
                # Prune *this node's* dead entries: same node, but the REPL that
                # registered them has exited (no /proc/<pid>). This keeps the file
                # bounded without touching live sibling REPLs on this node (their
                # pid is still alive) or other nodes' entries (we can't PID-check a
                # remote process — the hub prunes those after backoff).
                bn = hub_field(blk, "node")
                bpid = (pm = match(r"\"pid\"\s*:\s*(\d+)", blk); pm === nothing ? -1 : parse(Int, pm.captures[1]))
                (bn == node && bpid > 0 && !ispath("/proc/$bpid")) && continue
                push!(existing, "    " * strip(blk))
            end
        end
        push!(existing, _gate_json(; host, port, stream_port, token, name, node, pid))
        return open(GATES_JSON, "w") do io
            println(io, "{")
            println(io, "  \"tcp_gates\": [")
            println(io, join(existing, ",\n"))
            println(io, "  ]")
            println(io, "}")
        end
    end

    # Open a reverse SSH tunnel forwarding our local gate ports onto the hub's
    # loopback. Long-lived; tied to this REPL via atexit. Non-interactive & never
    # prompts (BatchMode); exits rather than hanging if a forward fails.
    function open_reverse_tunnel(hub_host, hub_rep, hub_pub, local_rep, local_pub)
        cmd = `ssh -N
        -o StrictHostKeyChecking=accept-new -o BatchMode=yes
        -o ExitOnForwardFailure=yes -o ServerAliveInterval=30 -o ServerAliveCountMax=3
        -R $(hub_rep):127.0.0.1:$(local_rep)
        -R $(hub_pub):127.0.0.1:$(local_pub)
        $(hub_host)`
        proc = run(pipeline(cmd; stdout = devnull, stderr = devnull); wait = false)
        atexit(
            () -> (
                try
                    kill(proc)
                catch end
            )
        )
        return proc
    end

    # Decide this REPL's federation role from hub.json. No side effects (no serve,
    # tunnel, or registration) — those happen in register!() AFTER Gate.serve has
    # bound its ephemeral ports. Returns nothing (no hub → skip federation), or a
    # NamedTuple with `skip::Bool` and either `msg` (skip reason) or the role fields.
    function decide()
        hub = read_hub()
        hub === nothing && return nothing
        hub_host = hub_field(hub, "host")
        token = hub_field(hub, "token")
        isempty(hub_host) && return nothing
        occursin("headnode", hub_host) &&
            return (; skip = true, msg = "refusing to register against headnode hub")

        me = gethostname()
        name = me * " (" * (
            try
                basename(dirname(Base.active_project()))
            catch
                "?"
            end
        ) * ")"
        is_hub = startswith(hub_host, me) || hub_host == me  # FQDN starts with short name
        return (; skip = false, token, hub_host, me, name, is_hub)
    end

    # Register a freshly-served gate. `local_rep`/`local_pub` are the ephemeral ports
    # Gate.serve actually bound on 127.0.0.1. Returns a status string.
    function register!(role, local_rep::Integer, local_pub::Integer)
        if role.is_hub
            # Hub node: the hub dials our loopback directly — register the ports as-is.
            upsert_gate!(
                host = "127.0.0.1", port = local_rep, stream_port = local_pub,
                token = role.token, name = role.name, node = role.me, pid = getpid()
            )
            return "hub-local gate on 127.0.0.1:$(local_rep)"
        else
            # Remote node: map our local ports onto a per-REPL-unique pair on the hub's
            # loopback. Key the pair on host+local_rep (not host alone) so two REPLs on
            # the same node get distinct hub-side ports — and distinct session ids.
            hrep, hpub = hub_port_pair(role.me * ":" * string(local_rep))
            proc = try
                open_reverse_tunnel(role.hub_host, hrep, hpub, local_rep, local_pub)
            catch e
                return "tunnel to $(role.hub_host) FAILED ($e); gate up locally but unregistered"
            end
            # Wait for SSH to authenticate and bind the reverse forwards *before*
            # advertising the gate, so the hub's first dial succeeds rather than
            # being refused (a refusal costs a 5s poll backoff that can blow past
            # the REPL's connect banner). ExitOnForwardFailure=yes ⇒ ssh exits if a
            # forward can't bind, so surviving the grace window means it's ready.
            for _ in 1:20                       # up to ~2s
                sleep(0.1)
                process_running(proc) ||
                    return "tunnel to $(role.hub_host) FAILED (forward refused); gate up locally but unregistered"
            end
            upsert_gate!(
                host = "127.0.0.1", port = hrep, stream_port = hpub,
                token = role.token, name = role.name, node = role.me, pid = getpid()
            )
            return "remote gate → hub $(role.hub_host) (loopback $hrep/$hpub)"
        end
    end
end  # module _KaimonFed

if isinteractive()
    try
        using Kaimon
        role = _KaimonFed.decide()
        if role === nothing
            @warn "Kaimon: no hub.json found; is the .bashrc hub-election block present? Gate not started."
        elseif role.skip
            @warn "Kaimon: $(role.msg)"
        else
            if !isempty(role.token)
                ENV["KAIMON_GATE_TOKEN"] = role.token
            end
            # Bind ephemeral loopback ports (port=0 → ZMQ picks free ports) so any
            # number of REPLs can run on one node without colliding. Read the actual
            # bound ports back, then register/tunnel them.
            Gate.serve(mode = :tcp, host = "127.0.0.1", port = 0, stream_port = 0)
            local_rep = Gate._TCP_PORT[]
            local_pub = Gate._TCP_STREAM_PORT[]
            status = _KaimonFed.register!(role, local_rep, local_pub)
            @info "Kaimon gate ($status)"
        end
    catch e
        @warn "Kaimon Gate failed to start" exception = e
    end
end

# Kaimon connection banner (TCP mode) — confirm the hub attached to THIS REPL.
# Gate.serve() advertises our session; the hub's poll loop dials our tunnel and
# health-pings every ~2s. A received ping (_PING_COUNT > 0) is the real proof of
# attachment, so we wait (async) for the first one and print the moment it lands —
# reactive, not a hard cutoff — only warning if still unpinged after a generous
# bound (cold hub start + tunnel + poll discovery can legitimately take a while).
# We no longer count "other REPLs" from SOCK_DIR — TCP gates leave no adverts there.
if isinteractive()
    @async try
        if @isdefined(Gate) && Gate._RUNNING[]
            sid = first(Gate._SESSION_ID[], 8)
            connected = false
            for _ in 1:160                     # wait up to ~40s; print as soon as pinged
                if Gate._PING_COUNT[] > 0
                    connected = true
                    break
                end
                sleep(0.25)
            end
            if connected
                printstyled(
                    "✓ Kaimon: this REPL is connected to the hub (session $sid)\n";
                    color = :green, bold = true
                )
            else
                printstyled(
                    "\n⚠ Kaimon: session $sid advertised but the hub has not pinged it yet. " *
                        "It may still attach shortly; if not, check the hub is up (:2828) " *
                        "and the SSH tunnel/registration succeeded.\n";
                    color = :yellow
                )
            end
        else
            printstyled(
                "\n○ Kaimon: gate not running in this REPL (Gate.serve() did not start)\n";
                color = :light_black
            )
        end
    catch
        # never let the banner break the REPL
    end
end
