#!/usr/bin/env bash
# Checks themes: theme.json fields, contract range, and a full compile (via `sim --check`).
# Usage: tools/check-themes.sh [theme-dir ...]   (default: every themes/*/ with a theme.json)
# SIM=/path/to/sim overrides the binary; otherwise sim/ is built in release mode first.
# Also checks refused fixtures (import guard and options validation), and every
# tools/fixtures/loads/* theme with the host overlay intact.
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
    expected="import guard"
    case "$(basename "$dir")" in
        options-bad) expected="theme.json: options[" ;;
        options-type) expected="host root failed to compile" ;;
    esac
    if out=$("$SIM" --check "$dir" --contract "$ROOT/contract" 2>&1); then
        echo "error: fixture $dir loaded; it must be refused" >&2
        status=1
    elif ! grep -Fq "$expected" <<<"$out"; then
        echo "error: fixture $dir failed, but not for $expected:" >&2
        echo "$out" >&2
        status=1
    else
        echo "ok: $(basename "$dir") refused: $(grep -Fm1 "$expected" <<<"$out")"
    fi
done

# Hostile themes the host must load but keep below the overlay. Probes: state, sim time, pixel,
# expected colour ("-" = anything but the theme's black). At t=13 of the ride the left
# indicator is lit (slot 0 icon, OverlayTokens.ind-color); a theme writing Vehicle.* must not blank it.
probes=("ride 13 1200,24 -" "ride 13 36,24 #5cff9d" "locked 5 600,280 -")
png="$(mktemp)"
clock_settings="$(mktemp)"
trap 'rm -f "$png" "$clock_settings"' EXIT
for dir in "$ROOT"/tools/fixtures/loads/*/; do
    dir="${dir%/}"
    if ! "$SIM" --check "$dir" --contract "$ROOT/contract" >&2; then
        status=1
        continue
    fi
    for probe in "${probes[@]}"; do
        read -r state at xy want <<<"$probe"
        got=$("$SIM" "$dir" --contract "$ROOT/contract" --state "$state" --at "$at" --screenshot "$png" --pixel "$xy" | tail -n 1) || { status=1; continue; }
        if [ "$want" = "-" ] && [ "$got" != "#000000" ] || [ "$got" = "$want" ]; then
            echo "ok: $(basename "$dir") overlay intact ($state t=$at, pixel $xy is $got)"
        else
            echo "error: fixture $dir covers or blanks the overlay ($state t=$at, pixel $xy is $got, want $want)" >&2
            status=1
        fi
    done
done
# Options use manifest defaults, then typed command-line overrides, inside Dash only.
for accent in default '#ff0000'; do
    flags=()
    want="#123456"
    if [ "$accent" != default ]; then
        flags=(--opt "accent=$accent" --opt enabled=false --opt count=-2 --opt style=thin)
        want="$accent"
    fi
    got=$("$SIM" "$ROOT/tools/fixtures/loads/options" --contract "$ROOT/contract" "${flags[@]}" --state park --at 5 --screenshot "$png" --pixel 640,384 | tail -n 1) || got=error
    if [ "$got" = "$want" ]; then
        echo "ok: theme option accent ($accent, pixel 640,384 is $got)"
    else
        echo "error: theme option accent ($accent, pixel 640,384 is $got, want $want)" >&2
        status=1
    fi
done
# Theme customization rows start at (432,176), 96 px apart. The second row's
# bool Choice starts at (848,280); (976,296) is inside On, off text and corners.
# The first row's eight swatches start at (564,184), with 60 px pitch; the default
# disc centre (596,216) stays #123456; (596,187) is inside its 3 px foreground ring.
# PR B's Dash probe at (640,384) is unchanged.
for probe in "976,296 #ffc94a" "596,216 #123456" "596,187 #0f2a2a"; do
    read -r xy want <<<"$probe"
    got=$("$SIM" "$ROOT/tools/fixtures/loads/options" --contract "$ROOT/contract" --state park --screen settings --pane theme --theme-view options --at 5 --screenshot "$png" --pixel "$xy" | tail -n 1) || got=error
    if [ "$got" = "$want" ]; then
        echo "ok: theme option control (pixel $xy is $got)"
    else
        echo "error: theme option control (pixel $xy is $got, want $want)" >&2
        status=1
    fi
done
if out=$("$SIM" "$ROOT/themes/default" --contract "$ROOT/contract" --opt accent=#ff0000 --at 5 --screenshot "$png" 2>&1); then
    if [ "$(grep -Fxc "theme option 'accent' skipped: not declared by the theme" <<<"$out")" -eq 1 ]; then
        echo "ok: undeclared accent skipped once; default theme still renders"
    else
        echo "error: default theme must log exactly one undeclared accent skip:" >&2
        echo "$out" >&2
        status=1
    fi
else
    echo "error: default theme failed with an undeclared option:" >&2
    echo "$out" >&2
    status=1
fi
# NO DATA badge: --stale must paint it red (OverlayTokens.red) at the left edge of its slot, off the text.
got=$("$SIM" "$ROOT/themes/minimal" --contract "$ROOT/contract" --stale --at 13 --screenshot "$png" --pixel 1120,24 | tail -n 1) || got=error
if [ "$got" = "#ff3b30" ]; then
    echo "ok: NO DATA badge shown with --stale (pixel 1120,24 is $got)"
else
    echo "error: --stale does not show the NO DATA badge (pixel 1120,24 is $got, want #ff3b30)" >&2
    status=1
fi
# Settings rail: parked opens the frame; riding closes it before drawing.
for state in park ride; do
    got=$("$SIM" "$ROOT/themes/minimal" --contract "$ROOT/contract" --state "$state" --screen settings --at 13 --screenshot "$png" --pixel 136,132 | tail -n 1) || got=error
    want="#ffc94a"
    [ "$state" = ride ] && want="#000000"
    if [ "$got" = "$want" ]; then
        echo "ok: settings gate ($state, pixel 136,132 is $got)"
    else
        echo "error: settings gate ($state, pixel 136,132 is $got, want $want)" >&2
        status=1
    fi
done
# Display choice: the default Off option uses the checked lock accent.
got=$("$SIM" "$ROOT/themes/minimal" --contract "$ROOT/contract" --state park --screen settings --pane display --at 13 --screenshot "$png" --pixel 736,192 | tail -n 1) || got=error
if [ "$got" = "#ffc94a" ]; then
    echo "ok: Display Off option selected (pixel 736,192 is $got)"
else
    echo "error: Display Off option not selected (pixel 736,192 is $got, want #ffc94a)" >&2
    status=1
fi
# (1200,600) is bare SettingsScreen background: right of the pane and below
# Close, above the footer, and clear of the strip. Minimal uses host lock defaults.
printf '%s\n' '{"night":"auto"}' > "$clock_settings"
for probe in "21:00 trusted #0b1c1c" "12:00 trusted #f6f3ec" "21:00 untrusted #f6f3ec"; do
    read -r hhmm trust want <<<"$probe"
    flags=()
    [ "$trust" = untrusted ] && flags=(--untrusted-clock)
    got=$("$SIM" "$ROOT/themes/minimal" --contract "$ROOT/contract" --settings "$clock_settings" --clock "$hhmm" "${flags[@]}" --state park --screen settings --pane display --at 5 --screenshot "$png" --pixel 1200,600 | tail -n 1) || got=error
    if [ "$got" = "$want" ]; then
        echo "ok: Auto night ($hhmm $trust, pixel 1200,600 is $got)"
    else
        echo "error: Auto night ($hhmm $trust, pixel 1200,600 is $got, want $want)" >&2
        status=1
    fi
done
# Three 420 px Choice pills start at x=652, y=176; Auto starts at x=942.67.
# (968,192) is inside Auto's accent fill, off text and corners, even during day.
got=$("$SIM" "$ROOT/themes/minimal" --contract "$ROOT/contract" --settings "$clock_settings" --clock 12:00 --state park --screen settings --pane display --at 5 --screenshot "$png" --pixel 968,192 | tail -n 1) || got=error
if [ "$got" = "#ffc94a" ]; then
    echo "ok: Display Auto option selected during day (pixel 968,192 is $got)"
else
    echo "error: Display Auto option not selected during day (pixel 968,192 is $got, want #ffc94a)" >&2
    status=1
fi
# Theme rows begin at (432,176), 96 px apart: Minimal is third after Default and
# Data Fields. (448,408) is in its left padding, clear of text and the 24 px corners.
got=$("$SIM" "$ROOT/themes/minimal" --contract "$ROOT/contract" --state park --screen settings --pane theme --at 13 --screenshot "$png" --pixel 448,408 | tail -n 1) || got=error
if [ "$got" = "#ffc94a" ]; then
    echo "ok: active theme row selected (pixel 448,408 is $got)"
else
    echo "error: active theme row not selected (pixel 448,408 is $got, want #ffc94a)" >&2
    status=1
fi
# Plugins rows start at (432,176), width 640 and height 80. Choice starts at
# (848,184); its On pill starts at (956,184), size 92 x 64. (976,200) is
# inside the enabled first row's On fill, clear of its text and rounded edge.
got=$("$SIM" "$ROOT/themes/minimal" --contract "$ROOT/contract" --state park --screen settings --pane plugins --plugins-file "$ROOT/tools/fixtures/plugins/installed.json" --at 13 --screenshot "$png" --pixel 976,200 | tail -n 1) || got=error
if [ "$got" = "#ffc94a" ]; then
    echo "ok: plugin On option selected (pixel 976,200 is $got)"
else
    echo "error: plugin On option not selected (pixel 976,200 is $got, want #ffc94a)" >&2
    status=1
fi
# First-run column: (200,208), 880 px wide; Done at (840,496), size 240 x 64.
# (864,528) is in the pill's left padding, clear of its text and rounded edge.
got=$("$SIM" "$ROOT/themes/minimal" --contract "$ROOT/contract" --first-run --at 5 --screenshot "$png" --pixel 864,528 | tail -n 1) || got=error
if [ "$got" = "#ffc94a" ]; then
    echo "ok: first-run Done pill visible (pixel 864,528 is $got)"
else
    echo "error: first-run Done pill missing (pixel 864,528 is $got, want #ffc94a)" >&2
    status=1
fi
# Switching uses the host day background, above the theme and below the overlay.
got=$("$SIM" "$ROOT/themes/minimal" --contract "$ROOT/contract" --switch default --at 5 --screenshot "$png" --pixel 104,104 | tail -n 1) || got=error
if [ "$got" = "#f6f3ec" ]; then
    echo "ok: switching card background (pixel 104,104 is $got)"
else
    echo "error: switching card missing (pixel 104,104 is $got, want #f6f3ec)" >&2
    status=1
fi
# After a theme restart the switching card holds the lock screen until lock state is known.
# The spinner steps every 125 ms: at t=2 (step 0) the top dot (640,354) is fully opaque.
got=$("$SIM" "$ROOT/themes/minimal" --contract "$ROOT/contract" --state restart --at 2 --screenshot "$png" --pixel 640,354 | tail -n 1) || got=error
if [ "$got" = "#0f2a2a" ]; then
    echo "ok: restart holds the switching card (pixel 640,354 is $got)"
else
    echo "error: restart shows the lock screen (pixel 640,354 is $got, want #0f2a2a)" >&2
    status=1
fi
# The theme contract stays HH:MM even when the host uses 12-hour time. At t=5 the
# theme clock is 10:24; 730,647 is inside the lit upper-right segment of its tens-hour 1.
printf '%s\n' '{"clock24": false}' > "$clock_settings"
got=$("$SIM" "$ROOT/themes/segment" --contract "$ROOT/contract" --settings "$clock_settings" --at 5 --screenshot "$png" --pixel 730,647 | tail -n 1) || got=error
if [ "$got" = "#1d201c" ]; then
    echo "ok: segment theme keeps 24-hour clock with clock24=false (pixel 730,647 is $got)"
else
    echo "error: segment clock missing with clock24=false (pixel 730,647 is $got, want #1d201c)" >&2
    status=1
fi
# Locked time is 18:24: the host shows 6:24 and a separate PM label at its baseline.
got=$("$SIM" "$ROOT/themes/minimal" --contract "$ROOT/contract" --settings "$clock_settings" --state locked --at 5 --screenshot "$png" --pixel 388,164 | tail -n 1) || got=error
if [ "$got" != error ] && [ "$got" != "#f6f3ec" ]; then
    echo "ok: lock clock has PM label with clock24=false (pixel 388,164 is $got)"
else
    echo "error: lock clock PM label missing with clock24=false (pixel 388,164 is $got)" >&2
    status=1
fi
# Hub end cells stay clear of all nine telltales in every strip placement.
for probe in "top 1220,24" "bottom 1220,744" "left 8,736" "right 1216,736"; do
    read -r placement xy <<<"$probe"
    got=$("$SIM" "$ROOT/themes/minimal" --contract "$ROOT/contract" --state park --placement "$placement" --at 13 --screenshot "$png" --pixel "$xy" | tail -n 1) || got=error
    if [ "$got" = "#ffc94a" ]; then
        echo "ok: hub end cell ($placement, pixel $xy is $got)"
    else
        echo "error: hub end cell ($placement, pixel $xy is $got, want #ffc94a)" >&2
        status=1
    fi
done
# Last pixel of the CHARGE label's box: slot-pad + 8 * slot-pitch-labels + slot + 6 + 74 - 1.
# From overlay.slint: 16 + 8 * 120 + 40 + 6 + 74 - 1 = 1095; strip centre y = 24.
got=$("$SIM" "$ROOT/themes/default" --contract "$ROOT/contract" --state park --placement top --at 13 --screenshot "$png" --pixel 1095,24 | tail -n 1) || got=error
if [ "$got" != error ] && [ "$got" != "#ffc94a" ]; then
    echo "ok: labelled CHARGE telltale clear of hub (pixel 1095,24 is $got)"
else
    echo "error: labelled CHARGE telltale covered by hub (pixel 1095,24 is $got, must not be #ffc94a)" >&2
    status=1
fi
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
