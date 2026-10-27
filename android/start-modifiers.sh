#!/usr/bin/env bash
set -euo pipefail

# Pass adb options such as -s SERIAL as arguments to this script.
adb "$@" shell '
set -e
apk=$(pm path io.github.dlip.chordpad)
apk=${apk#package:}
[ -n "$apk" ] || { echo "Install Chordpad first" >&2; exit 1; }
if pidof chordpad-modifiers >/dev/null; then
    echo "Chordpad modifier helper is already running"
    exit 0
fi
CLASSPATH="$apk" nohup app_process /system/bin --nice-name=chordpad-modifiers io.github.dlip.chordpad.ModifierHelper > /data/local/tmp/chordpad-modifiers.log 2>&1 < /dev/null &
'
for attempt in {1..30}; do
    if adb "$@" shell cat /data/local/tmp/chordpad-modifiers.log | tr -d '\r' | grep -q '^Chordpad global modifiers ready$'; then
        echo 'Global mouse modifiers connected. You can disconnect USB.'
        exit 0
    fi
    sleep 0.2
done
adb "$@" shell cat /data/local/tmp/chordpad-modifiers.log >&2
exit 1
