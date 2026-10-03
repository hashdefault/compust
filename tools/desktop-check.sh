#!/usr/bin/env bash
set -euo pipefail

report=${1:?Usage: tools/desktop-check.sh NEW_REPORT_DIRECTORY}
mkdir "$report"
report=$(realpath "$report")
work=$(mktemp -d /tmp/compust-desktop.XXXXXX)
pids=()
cleanup() {
    local pid
    for pid in "${pids[@]}"; do
        if kill -0 "$pid" 2>/dev/null; then
            kill -TERM "$pid" || true
        fi
        wait "$pid" || true
    done
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM

wait_display() {
    local path=$1 pid=$2
    for ((attempt = 0; attempt < 100; attempt++)); do
        if [[ -s "$path" ]]; then
            return
        fi
        kill -0 "$pid"
        sleep 0.05
    done
    printf 'X server did not publish its display number\n' >&2
    return 1
}

wm_binary="$work/xmonad-$(uname -m)-linux"
ghc -dynamic -O1 -outputdir "$work/ghc" -o "$wm_binary" \
    tools/desktop/xmonad.hs >"$report/xmonad-build.log" 2>&1
mkdir "$work/config" "$work/cache" "$work/data"
export XMONAD_CONFIG_DIR="$work/config"
export XMONAD_CACHE_DIR="$work/cache"
export XMONAD_DATA_DIR="$work/data"

"${XVFB:-Xvfb}" -displayfd 3 -screen 0 1280x800x24 -nolisten tcp -noreset \
    3>"$work/host-display" >"$report/xvfb.log" 2>&1 &
host_pid=$!
pids+=("$host_pid")
wait_display "$work/host-display" "$host_pid"
export DISPLAY=":$(<"$work/host-display")"

"${XEPHYR:-Xephyr}" -displayfd 3 -screen 1280x800 -nolisten tcp \
    3>"$work/display" >"$report/xephyr.log" 2>&1 &
server_pid=$!
pids=("$server_pid" "${pids[@]}")
wait_display "$work/display" "$server_pid"
export DISPLAY=":$(<"$work/display")"

"$wm_binary" >"$report/xmonad.log" 2>&1 &
wm_pid=$!
pids=("$wm_pid" "${pids[@]}")
for ((attempt = 0; attempt < 100; attempt++)); do
    if xprop -root _NET_SUPPORTING_WM_CHECK | grep -q 'window id'; then
        break
    fi
    kill -0 "$wm_pid"
    sleep 0.05
done
xprop -root _NET_SUPPORTING_WM_CHECK >"$report/wm.txt"
xdpyinfo >"$report/server.txt"
xrandr --current >"$report/outputs.txt" 2>&1
git rev-parse HEAD >"$report/commit.txt"
git status --short >"$report/worktree.txt"
uname -srmo >"$report/kernel.txt"
date -Is >"$report/started.txt"
lscpu >"$report/cpu.txt"
rustc --version >"$report/rust-version.txt"
xmonad --version >"$report/wm-version.txt"
cp tools/desktop/compust.toml "$report/compust.toml"
sha256sum target/release/compust target/release/examples/desktop_probe \
    tools/desktop/xmonad.hs "$wm_binary" \
    "$(command -v "${XVFB:-Xvfb}")" "$(command -v "${XEPHYR:-Xephyr}")" \
    >"$report/binaries.txt"

target/release/compust --display "$DISPLAY" --config "$report/compust.toml" \
    >"$report/compust.log" 2>&1 &
compositor_pid=$!
pids=("$compositor_pid" "${pids[@]}")
timeout 90s target/release/examples/desktop_probe --display "$DISPLAY" \
    --process "compust:$compositor_pid" --process "server:$server_pid" \
    --process "host:$host_pid" --process "wm:$wm_pid" \
    --seconds "${SECONDS_PER_PHASE:-10}" --output "$report" \
    >"$report/probe.log" 2>&1
kill -TERM "$compositor_pid"
wait "$compositor_pid"
pids=("$wm_pid" "$server_pid" "$host_pid")
printf 'Compositor exited successfully after SIGTERM.\n' >"$report/shutdown.txt"
printf 'Desktop checks passed: %s\n' "$report"
