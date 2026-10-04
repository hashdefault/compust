#!/usr/bin/env bash
set -euo pipefail

usage="Usage: startx \"\$PWD/tools/hardware-session.sh\" NEW_REPORT_ROOT [--features] -- :1"
if [[ ${1:-} == --help ]]; then
    printf '%s\n' "$usage"
    printf 'Run the present, direct, and effects desktop checks as the client of a dedicated\n'
    printf 'X session. SESSION_OUTPUT selects the one output left enabled. Build release binaries first.\n'
    printf 'With --features, check shadows, focus, and fullscreen suspension in XRender and GL instead.\n'
    exit 0
fi
if (( $# < 1 || $# > 2 )) || { (( $# == 2 )) && [[ $2 != --features ]]; }; then
    printf '%s\n' "$usage" >&2
    exit 2
fi
if [[ -z ${DISPLAY:-} ]]; then
    printf 'Run this script as the client of startx or xinit.\n' >&2
    exit 2
fi
root=$(realpath -m -- "$1")
cd -- "$(dirname -- "$(realpath -- "$0")")/.."
mkdir -- "$root"
exec > >(tee "$root/session.log") 2>&1
# Console logins may lack the rustup PATH used to build the project.
if [[ -d $HOME/.cargo/bin ]]; then
    PATH="$HOME/.cargo/bin:$PATH"
fi
# xinit starts the server and this client; another child of our parent is the server.
server_pid=${SERVER_PID:-$(pgrep -P "$PPID" | grep -vx "$$" | head -n 1)}
ps -o pid=,comm= -p "$server_pid"
# The server resets when its last client disconnects; keep one connected between runs.
xprop -root -spy WM_NAME >/dev/null &
keepalive=$!
trap 'kill "$keepalive" 2>/dev/null || true' EXIT

xrandr --current >"$root/outputs-initial.txt"
output=${SESSION_OUTPUT:-$(xrandr --current | awk '$2 == "connected" { print $1; exit }')}
# xrandr ignores a name it does not know, and the loop below would then turn every output off.
if ! xrandr --current | awk -v name="$output" '$1 == name && $2 == "connected" { found = 1 } END { exit !found }'; then
    printf 'Output %s is not connected. Connected outputs: %s\n' "${output:-(none)}" \
        "$(xrandr --current | awk '$2 == "connected" { printf "%s ", $1 }')" >&2
    exit 2
fi
layout=(--output "$output" --auto --primary --pos 0x0)
while read -r other; do
    layout+=(--output "$other" --off)
done < <(xrandr --current | awk -v keep="$output" '$2 == "connected" && $1 != keep { print $1 }')
xrandr "${layout[@]}"
xrandr --current >"$root/outputs.txt"
xrandr --listproviders >"$root/providers.txt"
if command -v lspci >/dev/null; then
    lspci -nnk -d ::0300 >"$root/gpu.txt"
fi
# Keep the screen awake; xset only reports a missing DPMS extension.
xset s off s noblank -dpms

status=0
if [[ ${2:-} == --features ]]; then
    if ! DESKTOP_DISPLAY=$DISPLAY SERVER_PID=$server_pid tools/features-check.sh "$root/features" both; then
        status=1
    fi
else
    for mode in present direct effects; do
        if DESKTOP_DISPLAY=$DISPLAY SERVER_PID=$server_pid tools/desktop-check.sh "$root/$mode" "$mode"; then
            printf '%s: passed\n' "$mode"
        else
            printf '%s: failed\n' "$mode"
            status=1
        fi
    done
fi
number=${DISPLAY#*:}
number=${number%%.*}
for log in "$HOME/.local/share/xorg/Xorg.$number.log" "/var/log/Xorg.$number.log"; do
    if [[ -r $log ]]; then
        cp -- "$log" "$root/xorg.log"
        break
    fi
done
printf 'Hardware session finished: %s\n' "$root"
exit "$status"
