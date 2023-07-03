" Vim syntax file
" Language: yggit
" Maintainer: Pierre-Louis
" Latest Revision: 1 July 2023

" Quit when a syntax file was already loaded.
if exists('b:current_syntax') | finish|  endif

syntax match commitName "\s\zs.*\ze"
syntax match commitHash "^\zs\x\+\ze" nextgroup=commitName skipwhite
syntax match branchSymbol "^->" nextgroup=branchName skipwhite
syntax match branchName "\s\zs.*\ze"
syntax match comment "^#.*"

hi def link branchSymbol Type
hi def link branchName Character
hi def link commitHash MoreMsg
hi def link commitName Function
hi def link comment Comment

let b:current_syntax = 'yggit'