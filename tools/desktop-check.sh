#!/usr/bin/env bash
set -euo pipefail

usage='Usage: tools/desktop-check.sh NEW_REPORT_DIRECTORY [present|direct|effects]'
if [[ ${1:-} == --help ]]; then
    printf '%s\n' "$usage"
    printf 'Run isolated Xmonad checks; Present is the default. Build release binaries first.\n'
    printf 'Set DESKTOP_DISPLAY and SERVER_PID to use an existing dedicated X server instead.\n'
    printf 'Set WINDOW_MANAGER to xmonad (the default) or openbox.\n'
    exit 0
fi
if (( $# < 1 || $# > 2 )); then
    printf '%s\n' "$usage" >&2
    exit 2
fi
mode=${2:-present}
opacity=100
case "$mode" in
    present) config=tools/desktop/compust.toml presentation=present ;;
    direct) config=tools/desktop/compust-direct.toml presentation=direct ;;
    effects) config=tools/desktop/compust-effects.toml presentation=present opacity=50 ;;
    *) printf 'Unknown mode: %s\n' "$mode" >&2; exit 2 ;;
esac
window_manager=${WINDOW_MANAGER:-xmonad}
case "$window_manager" in
    xmonad) layout=tiling ;;
    openbox) layout=stacking ;;
    *) printf 'Unknown window manager: %s\n' "$window_manager" >&2; exit 2 ;;
esac
if [[ -n ${DESKTOP_DISPLAY:-} && -z ${SERVER_PID:-} ]]; then
    printf 'SERVER_PID must identify the X server of DESKTOP_DISPLAY.\n' >&2
    exit 2
fi
report=$1
mkdir -- "$report"
report=$(realpath "$report")
work=$(mktemp -d /tmp/compust-desktop.XXXXXX)
# Processes this runner started, most recent first; an existing server is never listed.
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

mkdir "$work/config" "$work/cache" "$work/data"
if [[ $window_manager == xmonad ]]; then
    wm_source=tools/desktop/xmonad.hs
    wm_binary="$work/xmonad-$(uname -m)-linux"
    ghc -dynamic -O1 -outputdir "$work/ghc" -o "$wm_binary" \
        "$wm_source" >"$report/xmonad-build.log" 2>&1
    wm_command=("$wm_binary")
    export XMONAD_CONFIG_DIR="$work/config"
    export XMONAD_CACHE_DIR="$work/cache"
    export XMONAD_DATA_DIR="$work/data"
else
    wm_source=tools/desktop/openbox.xml
    wm_binary=$(command -v openbox)
    # Keep the user's own Openbox menus and session files out of the run.
    wm_command=(env "XDG_CONFIG_HOME=$work/config" "XDG_CACHE_HOME=$work/cache"
        "$wm_binary" --sm-disable --config-file "$wm_source")
fi

if [[ -n ${DESKTOP_DISPLAY:-} ]]; then
    export DISPLAY=$DESKTOP_DISPLAY
    server_pid=$SERVER_PID
    measured=(--process "server:$server_pid")
    servers=()
    ps -o pid=,args= -p "$server_pid" >"$report/server-process.txt"
    # A previous run's window manager can leave a stale EWMH check window behind.
    xprop -root -remove _NET_SUPPORTING_WM_CHECK
else
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
    measured=(--process "server:$server_pid" --process "host:$host_pid")
    servers=("$(command -v "${XVFB:-Xvfb}")" "$(command -v "${XEPHYR:-Xephyr}")")
fi

"${wm_command[@]}" >"$report/$window_manager.log" 2>&1 &
wm_pid=$!
pids=("$wm_pid" "${pids[@]}")
xdpyinfo >"$report/server.txt"
xrandr --current >"$report/outputs.txt" 2>&1
sources=(src examples tools Cargo.toml Cargo.lock rust-toolchain.toml)
git rev-parse HEAD >"$report/commit.txt"
git status --short >"$report/worktree.txt"
{
    git diff HEAD -- "${sources[@]}"
    while IFS= read -r -d '' file; do
        git diff --no-index -- /dev/null "$file" || [[ $? == 1 ]]
    done < <(git ls-files -z --others --exclude-standard -- "${sources[@]}")
} >"$report/source.patch"
git ls-files -z --cached --others --exclude-standard -- "${sources[@]}" \
    | xargs -0 sha256sum >"$report/source-sha256.txt"
uname -srmo >"$report/kernel.txt"
date -Is >"$report/started.txt"
lscpu >"$report/cpu.txt"
rustc --version >"$report/rust-version.txt"
# Read the whole output first: closing the pipe early would fail the run under pipefail.
wm_version=$("$window_manager" --version)
printf '%s\n' "${wm_version%%$'\n'*}" >"$report/wm-version.txt"
cp "$config" "$report/compust.toml"
sha256sum target/release/compust target/release/examples/desktop_probe \
    "$wm_source" "$wm_binary" "${servers[@]}" >"$report/binaries.txt"

target/release/compust --display "$DISPLAY" --config "$report/compust.toml" \
    >"$report/compust.log" 2>&1 &
compositor_pid=$!
pids=("$compositor_pid" "${pids[@]}")
timeout 90s target/release/examples/desktop_probe --display "$DISPLAY" \
    --process "compust:$compositor_pid" "${measured[@]}" --process "wm:$wm_pid" \
    --seconds "${SECONDS_PER_PHASE:-10}" --output "$report" --presentation "$presentation" \
    --opacity "$opacity" --layout "$layout" >"$report/probe.log" 2>&1
xprop -root _NET_SUPPORTING_WM_CHECK >"$report/wm.txt"
kill -TERM "$compositor_pid"
wait "$compositor_pid"
pids=("${pids[@]:1}")
printf 'Compositor exited successfully after SIGTERM.\n' >"$report/shutdown.txt"
printf 'Desktop checks passed: %s\n' "$report"
