syntax on

call plug#begin('~/.vim/plugged')

Plug 'powerman/vim-plugin-AnsiEsc'

call plug#end()

" Automatically enable ANSI color code highlighting for all files
autocmd BufRead,BufNewFile * AnsiEsc

" Set the default sort method in netrw to be by time
let g:netrw_sort_by = "time"
" Set the default sort order to be reversed (newest first)
let g:netrw_sort_direction = "reverse"

