#!/usr/bin/env bash
# Builds the Windows DLL from Linux (MinGW). Needs: rustup target x86_64-pc-windows-gnu, or
# rust-src for -Zbuild-std when that target can't be downloaded; and x86_64-w64-mingw32-gcc.
# Output: openkh/kh1-recom.dll
set -euo pipefail
cd "$(dirname "$0")/.."
export CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER=x86_64-w64-mingw32-gcc
if rustup target list --installed 2>/dev/null | grep -q x86_64-pc-windows-gnu; then
  cargo build --release --target x86_64-pc-windows-gnu -p kh1-recom
else
  RUSTC_BOOTSTRAP=1 cargo build --release --target x86_64-pc-windows-gnu -p kh1-recom \
    -Zbuild-std=std,panic_abort
fi
cp target/x86_64-pc-windows-gnu/release/kh1_recom.dll openkh/kh1-recom.dll
echo "openkh/kh1-recom.dll"
