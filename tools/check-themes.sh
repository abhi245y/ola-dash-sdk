#!/usr/bin/env bash
# Checks themes: theme.json fields, contract range, and a full compile (via `sim --check`).
# Usage: tools/check-themes.sh [theme-dir ...]   (default: every themes/*/ with a theme.json)
# SIM=/path/to/sim overrides the binary; otherwise sim/ is built in release mode first.
# Exits non-zero if any theme fails, so it can gate CI.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
if [ -z "${SIM:-}" ]; then
    cargo build --release --manifest-path "$ROOT/sim/Cargo.toml" >&2
    SIM="$ROOT/sim/target/release/sim"
fi

if [ "$#" -gt 0 ]; then
    themes=("$@")
else
    themes=("$ROOT"/themes/*/)
fi

status=0
checked=0
for dir in "${themes[@]}"; do
    dir="${dir%/}"
    if [ ! -f "$dir/theme.json" ]; then
        echo "skip: $dir (no theme.json)"
        continue
    fi
    checked=$((checked + 1))
    "$SIM" --check "$dir" --contract "$ROOT/contract" || status=1
done

if [ "$checked" -eq 0 ]; then
    echo "error: no themes with a theme.json found" >&2
    exit 1
fi
if [ "$status" -eq 0 ]; then
    echo "all $checked theme(s) passed"
else
    echo "theme check FAILED" >&2
fi
exit "$status"
