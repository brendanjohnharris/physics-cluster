#! /bin/bash

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
export JULIA_NUM_THREADS="auto"
export VSCODE_CLI_USE_FILE_KEYCHAIN=1
export WORKDIR="/taiji1/bhar9988"
export TERM="xterm-256color"
export PATH="$HOME/.local/bin/:$PATH"
export PATH="/usr/physics/pbspro/bin:$PATH"

if [[ $(hostname) =~ headnode* ]]; then
    export JULIA_NUM_THREADS="1"
fi
if [[ $(hostname) =~ node* ]]; then
    export JULIA_CONDAPKG_OFFLINE="yes"
    export ALLEN_NEUROPIXELS_OFFLINE="true"
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
module load pbspro
module load gsl-2.7
module load hdf/5/1.14.1-2_intel2021

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

