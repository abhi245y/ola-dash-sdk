#!/usr/bin/env bash
# Debug-build test channel. Usage: inject.sh <cb> <method> [args...] | inject.sh off
# cb: display|motor|modes|alerts|info|commands|battery|charger|handlebar. Needs adb root/shell; ADB_SERIAL optional.
# Examples: inject.sh display isTrunkLock false   (trunk shown open)
#           inject.sh display isTrunkLock true    (trunk shown closed)
set -euo pipefail
A=(adb ${ADB_SERIAL:+-s "$ADB_SERIAL"} shell)
if [ "$1" = off ]; then
    "${A[@]}" am broadcast -a dev.olatool.dash.TEST_OFF >/dev/null
    exit
fi
cb=$1 m=$2; shift 2
args=$(IFS=,; echo "$*")
"${A[@]}" am broadcast -a dev.olatool.dash.TEST --es cb "$cb" --es m "$m" ${args:+--esa a "$args"} >/dev/null
