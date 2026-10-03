#!/usr/bin/env bash
set -euo pipefail

if [[ ${1:-} == --help ]]; then
    printf 'Usage: tools/desktop-check.sh NEW_REPORT_DIRECTORY [present|direct]\n'
    printf 'Run isolated Xmonad checks; Present is the default. Build release binaries first.\n'
    exit 0
fi
if (( $# < 1 || $# > 2 )); then
    printf 'Usage: tools/desktop-check.sh NEW_REPORT_DIRECTORY [present|direct]\n' >&2
    exit 2
fi
presentation=${2:-present}
case "$presentation" in
    present) config=tools/desktop/compust.toml ;;
    direct) config=tools/desktop/compust-direct.toml ;;
    *) printf 'Unknown presentation mode: %s\n' "$presentation" >&2; exit 2 ;;
esac
report=$1
mkdir -- "$report"
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

wm_binary="$work/xmonad-$(uname -m)-linux"
ghc -dynamic -O1 -outputdir "$work/ghc" -o "$wm_binary" \
    tools/desktop/xmonad.hs >"$report/xmonad-build.log" 2>&1
mkdir "$work/config" "$work/cache" "$work/data"
export XMONAD_CONFIG_DIR="$work/config"
export XMONAD_CACHE_DIR="$work/cache"
export XMONAD_DATA_DIR="$work/data"

mkfifo "$work/host-display" "$work/display"
exec {host_ready}<>"$work/host-display"
"${XVFB:-Xvfb}" -displayfd 3 -screen 0 1280x800x24 -nolisten tcp -noreset \
    3>&"$host_ready" >"$report/xvfb.log" 2>&1 &
host_pid=$!
pids+=("$host_pid")
IFS= read -r -t 10 -u "$host_ready" host_display
exec {host_ready}>&-
export DISPLAY=":$host_display"

exec {server_ready}<>"$work/display"
"${XEPHYR:-Xephyr}" -displayfd 3 -screen 1280x800 -nolisten tcp \
    3>&"$server_ready" >"$report/xephyr.log" 2>&1 &
server_pid=$!
pids=("$server_pid" "${pids[@]}")
IFS= read -r -t 10 -u "$server_ready" server_display
exec {server_ready}>&-
export DISPLAY=":$server_display"

"$wm_binary" >"$report/xmonad.log" 2>&1 &
wm_pid=$!
pids=("$wm_pid" "${pids[@]}")
xdpyinfo >"$report/server.txt"
xrandr --current >"$report/outputs.txt" 2>&1
git rev-parse HEAD >"$report/commit.txt"
git status --short >"$report/worktree.txt"
git diff HEAD -- src examples tools Cargo.toml Cargo.lock rust-toolchain.toml >"$report/source.patch"
git ls-files -z src examples tools Cargo.toml Cargo.lock rust-toolchain.toml \
    | xargs -0 sha256sum >"$report/source-sha256.txt"
uname -srmo >"$report/kernel.txt"
date -Is >"$report/started.txt"
lscpu >"$report/cpu.txt"
rustc --version >"$report/rust-version.txt"
xmonad --version >"$report/wm-version.txt"
cp "$config" "$report/compust.toml"
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
    --seconds "${SECONDS_PER_PHASE:-10}" --output "$report" --presentation "$presentation" \
    >"$report/probe.log" 2>&1
xprop -root _NET_SUPPORTING_WM_CHECK >"$report/wm.txt"
kill -TERM "$compositor_pid"
wait "$compositor_pid"
pids=("$wm_pid" "$server_pid" "$host_pid")
printf 'Compositor exited successfully after SIGTERM.\n' >"$report/shutdown.txt"
printf 'Desktop checks passed: %s\n' "$report"
