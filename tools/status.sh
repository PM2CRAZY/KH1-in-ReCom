#!/usr/bin/env bash
# What is where: the repo's VERSION, the installed DLL's version, the version the game last ran
# (kh1-recom.log's first line) and when the asset step last wrote. Answers "which build did the
# tester run?" in one call.
#
# Environment: RECOM_GAME (the 1.5+2.5 game folder), KH1_RECOM_INSTALL (see dev-install.sh).
set -uo pipefail
cd "$(dirname "$0")/.."
game="${RECOM_GAME:-$HOME/Games/Steam/steamapps/common/KINGDOM HEARTS -HD 1.5+2.5 ReMIX-}"
dest="${KH1_RECOM_INSTALL:-$HOME/OpenKH/openkh/mods/Recom/KH1-in-ReCom/openkh/kh1-recom.dll}"
stamp() { [ -e "$1" ] && date -r "$1" '+%Y-%m-%d %H:%M' || echo "missing"; }
echo "repo:      $(cat VERSION) on $(git rev-parse --abbrev-ref HEAD) ($(git status --porcelain | wc -l) changed files)"
echo "installed: $(grep -a -o 'mod v0\.[0-9]*\.[0-9]*' "$dest" 2>/dev/null | head -1 | sed 's/mod v//') ($(stamp "$dest"))"
echo "game ran:  $(head -1 "$game/kh1-recom.log" 2>/dev/null | sed 's/kh1-recom //') ($(stamp "$game/kh1-recom.log"))"
echo "assets:    built $(stamp "$game/kh1-recom-generated")$( [ -n "${1:-}" ] && echo "; $1: $(stamp "$game/kh1-recom-generated/$1")")"
