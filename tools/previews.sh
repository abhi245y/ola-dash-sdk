#!/usr/bin/env bash
# Regenerate riding previews for SDK themes and optional product themes.
# SIM=/path/to/sim uses an existing binary; otherwise build the release sim first.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
if [ -z "${SIM:-}" ]; then
    cargo build --release --manifest-path "$ROOT/sim/Cargo.toml" >&2
    SIM="$ROOT/sim/target/release/sim"
fi

themes=("$ROOT"/themes/*/)
if [ -n "${EXTRA_THEMES:-}" ]; then
    themes+=("$EXTRA_THEMES"/*/)
fi
for dir in "${themes[@]}"; do
    [ -f "$dir/theme.json" ] || continue
    "$SIM" "${dir%/}" --contract "$ROOT/contract" --write-preview
done
