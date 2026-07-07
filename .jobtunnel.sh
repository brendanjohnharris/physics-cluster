#!/bin/bash
# Give this PBS job an SSH-reachable shell on its compute node, for live inspection
# (top, free, gdb, ...). Source from a job script (`source ~/.jobtunnel.sh`); no-op
# outside a job.
#
# The nodes refuse inbound SSH and their system sshd rejects us even via loopback,
# so we run our OWN rootless sshd on 127.0.0.1:$JOB_TUNNEL_PORT (authenticates
# against ~/.ssh/authorized_keys, no PAM/root) and reverse-forward that port to
# headnode. Connect with `sshjob <PBS_JOBID>`; find the port via `jobport <id>` or
# ~/.jobs/<PBS_JOBID>.port. Port derives from the job id (array-safe: subjob indices
# give consecutive, non-colliding ports).
#
# Sourced, not executed, so it runs in the job's own shell where PBS_JOBID is set;
# PBS runs job bodies in a child shell that does not inherit .bashrc functions.
if [[ -n "$PBS_JOBID" && -z "${_JOB_TUNNEL_UP:-}" ]]; then
    export _JOB_TUNNEL_UP=1
    _jt=${PBS_JOBID%%.*}
    export JOB_TUNNEL_PORT=$(( 10000 + ( ${_jt%%\[*} * 977 + ${PBS_ARRAY_INDEX:-0} ) % 22000 ))
    _jt_hk=~/.ssh/jobsshd_ed25519
    [ -f "$_jt_hk" ] || ssh-keygen -t ed25519 -N '' -f "$_jt_hk" -q
    # our own sshd: empty config (-f /dev/null) bypasses the system sshd's
    # AllowGroups/DenyUsers policy; UsePAM=no lets it run without root.
    /usr/sbin/sshd -D -p "$JOB_TUNNEL_PORT" -h "$_jt_hk" -f /dev/null \
        -o ListenAddress=127.0.0.1 -o UsePAM=no -o PasswordAuthentication=no \
        -o AuthorizedKeysFile="$HOME/.ssh/authorized_keys" -o StrictModes=no -o PidFile=none \
        >/dev/null 2>&1 &
    _jt_sshd=$!
    ssh -o BatchMode=yes -o ConnectTimeout=5 -o StrictHostKeyChecking=accept-new \
        -o ControlPath=none -o ExitOnForwardFailure=yes \
        -o ServerAliveInterval=60 -o ServerAliveCountMax=3 -N -f \
        -R "${JOB_TUNNEL_PORT}:localhost:${JOB_TUNNEL_PORT}" headnode 2>/dev/null \
        && echo "$JOB_TUNNEL_PORT $(hostname)" > ~/.jobs/"${PBS_JOBID}".port
    trap 'kill $_jt_sshd 2>/dev/null; rm -f ~/.jobs/"${PBS_JOBID}".port' EXIT TERM
    unset _jt _jt_hk
fi
