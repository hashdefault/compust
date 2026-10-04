#!/usr/bin/env bash
# Compust before and after skipping hidden windows: every scene, builds alternating, twice per layout.
# BEFORE names a worktree of the previous commit; its own runner and binaries measure it.
set -uo pipefail
cd "$(git rev-parse --show-toplevel)"
out=$(realpath -m "${REPORTS:-artifacts}/bench-occlusion")
before=$(realpath "${BEFORE:-target/before}")
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
(cd "$before" && cargo build --release --locked --bin compust --example desktop_probe) \
    >"$out/build-before.log" 2>&1
export SERVER_PID WM_PID SECONDS_PER_PHASE=20 COMPOSITORS=compust
SERVER_PID=$(pgrep -x Xorg)
WM_PID=$(pgrep -x dwm)
measure() {
    local layout=$1
    for run in 1 2; do
        (cd "$before" && tools/bench.sh "$out/$layout-$run-before") >"$out/$layout-$run-before.log" 2>&1
        echo "$layout-$run-before exit=$?"
        tools/bench.sh "$out/$layout-$run-after" >"$out/$layout-$run-after.log" 2>&1
        echo "$layout-$run-after exit=$?"
    done
}
xrandr --output HDMI-1 --auto --right-of DP-2
sleep 3
measure dual
xrandr --output HDMI-1 --off
sleep 3
measure single
