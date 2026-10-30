#!/usr/bin/env bash
# Checks themes: theme.json fields, contract range, and a full compile (via `sim --check`).
# Usage: tools/check-themes.sh [theme-dir ...]   (default: every themes/*/ with a theme.json)
# SIM=/path/to/sim overrides the binary; otherwise sim/ is built in release mode first.
# Also requires the import guard to refuse every tools/fixtures/refused/* theme, and every
# tools/fixtures/loads/* theme to load with the host overlay intact (hostile on purpose).
# Exits non-zero if any theme fails or a fixture loads, so it can gate CI.
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

for dir in "$ROOT"/tools/fixtures/refused/*/; do
    dir="${dir%/}"
    if out=$("$SIM" --check "$dir" --contract "$ROOT/contract" 2>&1); then
        echo "error: fixture $dir loaded; the import guard must refuse it" >&2
        status=1
    elif ! grep -q "import guard" <<<"$out"; then
        echo "error: fixture $dir failed, but not in the import guard:" >&2
        echo "$out" >&2
        status=1
    else
        echo "ok: $(basename "$dir") refused: $(grep -m1 "import guard" <<<"$out")"
    fi
done

# Hostile themes the host must load but keep below the overlay: the strip (ride) and a keypad
# key (locked) must not show the theme's black.
png="$(mktemp)"
for dir in "$ROOT"/tools/fixtures/loads/*/; do
    dir="${dir%/}"
    if ! "$SIM" --check "$dir" --contract "$ROOT/contract" >&2; then
        status=1
        continue
    fi
    for probe in "ride 1200,24" "locked 710,200"; do
        read -r state xy <<<"$probe"
        got=$("$SIM" "$dir" --contract "$ROOT/contract" --state "$state" --screenshot "$png" --pixel "$xy" | tail -n 1)
        if [ "$got" = "#000000" ]; then
            echo "error: fixture $dir covers the overlay ($state, pixel $xy is $got)" >&2
            status=1
        else
            echo "ok: $(basename "$dir") overlay intact ($state, pixel $xy is $got)"
        fi
    done
done
rm -f "$png"

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
