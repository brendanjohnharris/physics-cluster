#	This is the default standard profile provided to a user.
#	They are expected to edit it to meet their own needs.

# MAIL=/usr/mail/${LOGNAME:?}
export TERM="xterm-256color"
export PATH="$HOME/.conda/envs/bhar9988/bin:/taiji1/bhar9988/build/vscode/bin/:$PATH"
export PATH="$HOME/.conda/envs/LaTeX/bin:$HOME/build/julia-1.9.0/bin:/usr/physics/pbspro/bin:$PATH"
export LD_LIBRARY_PATH="$HOME/.conda/envs/bhar9988/lib/:$LD_LIBRARY_PATH"

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
