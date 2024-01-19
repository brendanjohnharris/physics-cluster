# alias module='eval /usr/physics/Modules/3.2.8/bin/modulecmd bash'
# alias code='$HOME/build/vscode/bin/code --no-sandbox'

alias julia='$HOME/build/julia-1.10.0/bin/julia'
module load pbspro
# module load gsl-2.7
# module load Anaconda3-5.1.0
module load hdf/5/1.14.1-2_intel2021

# setenv LD_LIBRARY_PATH ""
# setenv LD_PRELOAD=""
# setenv JULIA_NUM_THREADS="auto"
# setenv CONDA_DEFAULT_ENV="bhar9988"
# set -Ux LD_LIBRARY_PATH ""

# fish_add_path /headnode2/bhar9988/.conda/envs/bhar9988/bin:/headnode2/bhar9988/build/vscode/bin/
# fish_add_path $HOME/build/julia-1.9.0/bin
set -U fish_user_paths /usr/physics/python/anaconda3/bin/ $fish_user_paths
set -U fish_user_paths /headnode2/bhar9988/.conda/envs/bhar9988/bin/ $fish_user_paths
set -U fish_user_paths /headnode2/bhar9988/build/julia-1.10.0/bin $fish_user_paths
set TERM xterm-256color

function fish_user_key_bindings
    bind "[A"  up-or-search
    bind "[B"  down-or-search
    bind "[C"  forward-char
    bind "[D" backward-char
    bind "[3~" delete-char
    bind "[1;5C" forward-word
    bind "[1;5D" backward-word
    bind "[200~" __fish_start_bracketed_paste
    bind "[201~" __fish_stop_bracketed_paste
end


function config
    git --git-dir=$HOME/physics-cluster --work-tree=$HOME $argv
end
