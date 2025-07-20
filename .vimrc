syntax on

call plug#begin('~/.vim/plugged')

Plug 'powerman/vim-plugin-AnsiEsc'

call plug#end()

" Automatically enable ANSI color code highlighting for all files
autocmd BufRead,BufNewFile * AnsiEsc

