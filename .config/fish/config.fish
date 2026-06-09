starship init fish | source

set TERM xterm-256color

function fish_user_key_bindings
    bind \e\[A up-or-search
    bind \e\[B down-or-search
    bind \e\[C forward-char
    bind \e\[D backward-char
    bind \e\[3~ delete-char
    bind \e\[1\;5C forward-word
    bind \e\[1\;5D backward-word
    bind \e\[200~ __fish_start_bracketed_paste
    bind \e\[201~ __fish_stop_bracketed_paste
end

set fish_greeting ""

# Color scheme
set fish_color_command 51afef
set fish_color_normal normal
set fish_color_quote 98be65
set fish_color_redirection 46d9ff
set fish_color_end da8548
set fish_color_error ff6c6b
set fish_color_param c678dd
set fish_color_comment 6d757d
set fish_color_match normal
set fish_color_selection dfdfdf
set fish_color_search_match ecbe7b
set fish_color_history_current normal
set fish_color_operator 4db5bd
set fish_color_escape 4db5bd
set fish_color_cwd 98be65
set fish_color_cwd_root 3071db
set fish_color_valid_path normal
set fish_color_autosuggestion 5699af
set fish_color_user 98be65
set fish_color_host normal
set fish_color_cancel normal
set fish_pager_color_completion normal
set fish_pager_color_description ecbe7b yellow
set fish_pager_color_prefix normal --bold --underline
set fish_pager_color_progress brwhite --background=cyan


function config
    git --git-dir=$HOME/physics-cluster --work-tree=$HOME $argv
end


# >>> juliaup initialize >>>
fish_add_path /suphys/bhar9988/.juliaup/bin
fish_add_path /suphys/bhar9988/.juliaup/bin/julia
# <<< juliaup initialize <<<

# Pixi
export PATH="/suphys/bhar9988/.pixi/bin:$PATH"