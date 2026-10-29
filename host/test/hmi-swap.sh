#!/usr/bin/env bash
# Swap the scooter's home screen between the stock HMI and ola-dash.
#   hmi-swap.sh dash     disable stock HMI, make ola-dash home, show it
#   hmi-swap.sh stock    re-enable stock HMI, make it home again
#   hmi-swap.sh status
# Needs the root adb session. ADB_SERIAL optional.
set -euo pipefail
A=(adb ${ADB_SERIAL:+-s "$ADB_SERIAL"} shell)
STOCK=com.olaelectric.hmiclient/.MainActivity
DASH=dev.olatool.dash/.MainActivity
case "${1:-status}" in
    dash)
        "${A[@]}" "pm disable-user --user 0 com.olaelectric.hmiclient \
            && cmd package set-home-activity $DASH && input keyevent KEYCODE_HOME"
        ;;
    stock)
        "${A[@]}" "pm enable com.olaelectric.hmiclient && cmd package set-home-activity $STOCK \
            && am force-stop dev.olatool.dash && input keyevent KEYCODE_HOME"
        ;;
    status)
        "${A[@]}" "pm list packages -d | grep -c hmiclient | sed 's/^/stock disabled: /'; \
            cmd package resolve-activity --brief -a android.intent.action.MAIN -c android.intent.category.HOME | tail -1"
        ;;
    *) echo "usage: $0 dash|stock|status" >&2; exit 2 ;;
esac
