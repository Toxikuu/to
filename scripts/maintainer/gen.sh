#!/bin/bash
# Script to (help) generate a package from its package file
# shellcheck disable=2154,1090

# pkg looks like n@v-r
tource "$1"

# export the variables
echo "$n"
echo "$v"
echo "$r"
echo "$a"
echo "$m"
(IFS=$'\x1f'; echo "${l[*]}")
echo "$u"
echo "$vf"
echo "${t[*]}" # tags are space-delimited
(IFS=$'\x1f'; echo "${s[*]}")
(IFS=$'\x1f'; echo "${d[*]}")
(IFS=$'\x1f'; echo "${kcfg[*]}")
