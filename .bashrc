#! /bin/bash
# .bashrc

# Source global definitions
if [ -f /etc/bashrc ]; then
    . /etc/bashrc
fi
export LD_LIBRARY_PATH=""
export LD_PRELOAD=""
export JULIA_NUM_THREADS="auto"
export VSCODE_CLI_USE_FILE_KEYCHAIN=1

export WORKDIR="/taiji1/bhar9988"

module load pbspro
module load gsl-2.7
module load hdf/5/1.14.1-2_intel2021
export CONDA_DEFAULT_ENV="bhar9988"
export PATH="$HOME/.local/bin/:$PATH"
export PATH="/usr/physics/pbspro/bin:$PATH"
test -e "$HOME/.shellfishrc" && source "$HOME/.shellfishrc"

export CONDA_DEFAULT_ENV="bhar9988"

config() {
    git --git-dir="$HOME/physics-cluster" --work-tree="$HOME" "$@"
}

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

if [[ $(hostname) =~ headnode* ]]; then
    export JULIA_NUM_THREADS="1"
fi

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
