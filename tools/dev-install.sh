#!/usr/bin/env bash
# Local loop for a change that is ready to try in game: the affected tests, the Windows DLL, the DLL
# installed into the OpenKH mod folder, its version checked. Prints a short summary; full logs go to
# target/dev-install/*.log.
#
#   tools/dev-install.sh            # tests + build + install
#   tools/dev-install.sh --no-test  # build + install only
#
# Machine paths (from the environment, e.g. ~/.bashrc.d/kh.sh):
#   KH1_RECOM_INSTALL  the DLL the game loads through OpenKH
#                      (default ~/OpenKH/openkh/mods/Recom/KH1-in-ReCom/openkh/kh1-recom.dll)
# The repo's own openkh/kh1-recom.dll is left alone (the build bot commits it).
set -euo pipefail
cd "$(dirname "$0")/.."

run_tests=1
[ "${1:-}" = "--no-test" ] && run_tests=0
logs=target/dev-install
mkdir -p "$logs"
want="$(cat VERSION)"
dest="${KH1_RECOM_INSTALL:-$HOME/OpenKH/openkh/mods/Recom/KH1-in-ReCom/openkh/kh1-recom.dll}"

if [ $run_tests = 1 ]; then
  if tools/test-affected.sh >"$logs/test.log" 2>&1; then
    passed=$(grep -o '[0-9]* passed' "$logs/test.log" | awk '{s+=$1} END {print s+0}')
    echo "tests: ok ($passed passed)"
  else
    echo "tests: FAILED (see $logs/test.log)"; grep -E 'panicked|FAILED|^error' "$logs/test.log" | head -10; exit 1
  fi
fi

export CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER=x86_64-w64-mingw32-gcc
# KH1_DLL_PROFILE (machine-local, e.g. "testing" defined in ~/.cargo/config.toml): a faster build for in-game tests;
# unset = the release profile, as the GitHub release builds it.
profile="${KH1_DLL_PROFILE:-release}"
# `--features dev`: development-only debug aids (`crates/kh1-recom/Cargo.toml`); release builds (`tools/build-win.sh`) never have them.
if cargo build --profile "$profile" --target x86_64-pc-windows-gnu -p kh1-recom --features dev >"$logs/build.log" 2>&1; then
  warns=$(grep -c '^warning' "$logs/build.log" || true)
  echo "build: ok ($profile, dev features, $warns warning lines)"
else
  echo "build: FAILED (see $logs/build.log)"; grep -E '^error' -A5 "$logs/build.log" | head -20; exit 1
fi

cp "target/x86_64-pc-windows-gnu/$profile/kh1_recom.dll" "$dest"
got="$(grep -a -o 'mod v0\.[0-9]*\.[0-9]*' "$dest" | head -1 | sed 's/mod v//')"
if [ "$got" = "$want" ]; then
  echo "install: $dest = $got (press Build in the Mods Manager if assets changed)"
else
  echo "install: VERSION MISMATCH: VERSION says $want, the installed DLL says $got"; exit 1
fi
