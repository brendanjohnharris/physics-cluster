#! /bin/bash

# * Source global definitions
if [ -f /etc/bashrc ]; then
    . /etc/bashrc
fi
if [ -f "$HOME/.shellfishrc" ]; then
    . "$HOME/.shellfishrc"
fi

# * Environment variables
export SM_THETA="(6, 10)"
export LD_LIBRARY_PATH=""
export LD_PRELOAD=""
export JULIA_NUM_THREADS="auto"
export VSCODE_CLI_USE_FILE_KEYCHAIN=1
export WORKDIR="/import/taiji1/bhar9988"
export TERM="xterm-256color"
export PATH="$HOME/.local/bin/:$PATH"
export PATH="/usr/physics/pbspro/bin:$PATH"
export PAGER="vim -R +AnsiEsc"
export JULIA_CONDAPKG_OFFLINE="yes"
export ALLEN_NEUROPIXELS_OFFLINE="true"
# export PYTHONWARNINGS="ignore::ImportWarning,ignore::UserWarning,ignore::DeprecationWarning"
# export PYTHONWARNINGS="ignore:JuliaCompatHooks.find_spec() not found:ImportWarning,ignore:pkg_resources is deprecated as an API:UserWarning"

# * Julia configuration
export JULIA_CPU_TARGET="generic;icelake-client,clone_all;haswell,clone_all;broadwell,clone_all;sandybridge,clone_all;znver3,clone_all;sapphirerapids,clone_all"
export JULIA_PKG_USE_CLI_GIT=true
export LD_LIBRARY_PATH=""
export FREETYPE_ABSTRACTION_FONT_PATH="/suphys/bhar9988/build/miniforge3/envs/bhar9988/fonts/"
export PATH="/suphys/bhar9988/build/miniforge3/envs/bhar9988/bin:$PATH"
export PYTHON="/suphys/bhar9988/build/miniforge3/envs/bhar9988/bin/python"
export JULIA_DEBUG="SpatiotemporalMotifs" # loading,VSCodeServer
export JULIA_WORKER_TIMEOUT=600
export USYDCLUSTERS_LOGDIR="/suphys/bhar9988/.jobs/"
export DRWATSON_STOREPATCH=true
# export JULIA_CONDAPKG_OFFLINE="yes"
# export JULIA_CONDAPKG_BACKEND="MicroMamba"
# export JULIA_PYTHONCALL_EXE="@PyCall"
# export FORESIGHT_PATCHES= true
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
alias qlload='set ncols=`tput cols`; env COLUMNS=200 qload | fold -c"$ncols"'
alias code-physics='$HOME/code-physics/start-code-physics'
alias code-physics-2='$HOME/code-physics/start-code-physics --workdir $HOME/code-physics/cli2/ --name code-physics-2'
config() {
    git --git-dir="$HOME/physics-cluster" --work-tree="$HOME" "$@"
}

# * Common modules
# module load pbspro
# module load gsl-2.7
# module load hdf/5/1.14.1-2_intel2021

# >>> conda initialize >>>
# !! Contents within this block are managed by 'conda init' !!
__conda_setup="$('$HOME/build/miniforge3/bin/conda' 'shell.bash' 'hook' 2> /dev/null)"
if [ $? -eq 0 ]; then
    eval "$__conda_setup"
else
    if [ -f "$HOME/build/miniforge3/etc/profile.d/conda.sh" ]; then
        . "$HOME/build/miniforge3/etc/profile.d/conda.sh"
    else
        export PATH="$HOME/build/miniforge3/bin:$PATH"
    fi
fi
unset __conda_setup
# <<< conda initialize <<<

conda activate bhar9988

# >>> juliaup initialize >>>

# !! Contents within this block are managed by juliaup !!

case ":$PATH:" in
    *:/suphys/bhar9988/.juliaup/bin:*)
        ;;

    *)
        export PATH=/suphys/bhar9988/.juliaup/bin${PATH:+:${PATH}}
        ;;
esac

# <<< juliaup initialize <<<

