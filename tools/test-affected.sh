#!/usr/bin/env bash
# Runs the tests of the crates a change touches, plus every workspace crate that depends on them
# (from `cargo metadata`), instead of `cargo test --workspace`.
#
#   tools/test-affected.sh                  # uncommitted changes (staged, unstaged, untracked)
#   tools/test-affected.sh --since <rev>    # also everything committed since <rev>
#                                           # (before a push: --since origin/<branch>)
#   tools/test-affected.sh --list ...       # print the crates, run nothing
#
# A change to the root Cargo.toml tests the whole workspace (Cargo.lock alone does not: it changes
# with every version bump). Files outside crates/
# (docs, tools, openkh/) test nothing. Extra arguments after `--` go to `cargo test`.
set -euo pipefail
cd "$(dirname "$0")/.."

since="" list=0 extra=()
while [ $# -gt 0 ]; do
  case "$1" in
    --since) since="$2"; shift 2 ;;
    --list) list=1; shift ;;
    --) shift; extra=("$@"); break ;;
    *) echo "unknown argument: $1" >&2; exit 2 ;;
  esac
done

changed="$( { git diff --name-only HEAD; git ls-files --others --exclude-standard;
              if [ -n "$since" ]; then git diff --name-only "$since" HEAD; fi; } | sort -u )"

crates="$(cargo metadata --format-version 1 --no-deps | python3 -I -c '
import json, os, sys
meta = json.load(sys.stdin)
changed = [l for l in sys.argv[1].splitlines() if l]
root = meta["workspace_root"]
pkgs = {p["name"]: os.path.relpath(os.path.dirname(p["manifest_path"]), root) for p in meta["packages"]}
# Cargo.lock changes with every version bump (the versions of the workspace crates): no reason to
# test everything. A real dependency change shows in the Cargo.toml of a crate, mapped to it.
if "Cargo.toml" in changed:
    print("--workspace"); sys.exit()
deps = {p["name"]: {d["name"] for d in p["dependencies"] if d["name"] in pkgs} for p in meta["packages"]}
hit = {n for n, d in pkgs.items() for f in changed if f == d or f.startswith(d + "/")}
grew = True
while grew:
    more = {n for n, ds in deps.items() if ds & hit} - hit
    grew = bool(more)
    hit |= more
print(" ".join(sorted(hit)))
' "$changed")"

if [ -z "$crates" ]; then
  echo "test-affected: no crate changed; nothing to test"
  exit 0
fi
echo "test-affected: $crates"
[ "$list" = 1 ] && exit 0
# cargo-nextest when installed (each test in its own process, run in parallel; KH1_TEST_RUNNER=cargo forces `cargo test`).
# It does not run doc tests; `cargo test --doc` does.
run=(cargo test)
if [ "${KH1_TEST_RUNNER:-}" != cargo ] && cargo nextest --version >/dev/null 2>&1; then
  run=(cargo nextest run --no-fail-fast --status-level fail --final-status-level fail)
fi
if [ "$crates" = "--workspace" ]; then
  exec "${run[@]}" --workspace "${extra[@]}"
fi
args=()
for c in $crates; do args+=(-p "$c"); done
exec "${run[@]}" "${args[@]}" "${extra[@]}"
