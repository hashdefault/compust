#!/usr/bin/env bash
# Compust after repainting only for visible changes: the scenes it affects, twice per layout.
set -uo pipefail
cd "$(git rev-parse --show-toplevel)"
S=${REPORTS:-artifacts}
out=$S/bench-repaint
mkdir -p "$out"
restore() {
    xrandr --output HDMI-1 --auto --right-of DP-2 || true
    sleep 2
    if ! pgrep -x compust >/dev/null && ! pgrep -x picom >/dev/null; then
        setsid -f target/release/compust >"$HOME/.local/state/compust.log" 2>&1 </dev/null
    fi
    date -Is >"$out/done.txt"
}
trap restore EXIT
pkill -TERM -x compust && sleep 1
cargo build --release --locked --bin compust --example desktop_probe >"$out/build.log" 2>&1
export SERVER_PID WM_PID SECONDS_PER_PHASE=20 COMPOSITORS=compust
export SCENES="small-update move-resize open-close"
SERVER_PID=$(pgrep -x Xorg)
WM_PID=$(pgrep -x dwm)
xrandr --output HDMI-1 --auto --right-of DP-2
sleep 3
for run in 1 2; do
    tools/bench.sh "$out/dual-$run" >"$out/dual-$run.log" 2>&1
    echo "dual-$run exit=$?"
done
xrandr --output HDMI-1 --off
sleep 3
for run in 1 2; do
    tools/bench.sh "$out/single-$run" >"$out/single-$run.log" 2>&1
    echo "single-$run exit=$?"
done
