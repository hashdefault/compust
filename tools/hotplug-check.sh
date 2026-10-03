#!/usr/bin/env bash
set -euo pipefail

usage() {
    printf 'Usage: SERVER_PID=PID [WM_PID=PID] tools/hotplug-check.sh NEW_REPORT_DIRECTORY [present|direct]\n'
}
if [[ ${1:-} == --help ]]; then
    usage
    printf 'Sample monitor transitions on DISPLAY after stopping its compositor; Present is the default.\n'
    printf 'Enter "sample LABEL" after each transition, then "quit". Build release binaries first.\n'
    exit 0
fi
if (( $# < 1 || $# > 2 )); then
    usage >&2
    exit 2
fi
presentation=${2:-present}
case "$presentation" in
    present) config=tools/desktop/compust.toml ;;
    direct) config=tools/desktop/compust-direct.toml ;;
    *) printf 'Unknown presentation mode: %s\n' "$presentation" >&2; exit 2 ;;
esac
if [[ -z ${DISPLAY:-} || -z ${SERVER_PID:-} ]]; then
    usage >&2
    printf 'DISPLAY and SERVER_PID must identify the dedicated test session.\n' >&2
    exit 2
fi
report=$1
mkdir -- "$report"
report=$(realpath "$report")
measured=(--process "server:$SERVER_PID")
if [[ -n ${WM_PID:-} ]]; then
    measured+=(--process "wm:$WM_PID")
fi
# Monitor EDIDs contain serial numbers; keep the other output properties.
outputs() {
    xrandr --verbose | awk '/^\t[^\t]/ { edid = /^\tEDID:/ } !(edid && /^\t\t/)'
}
compositor_pid=
cleanup() {
    if [[ -n $compositor_pid ]] && kill -0 "$compositor_pid" 2>/dev/null; then
        kill -TERM "$compositor_pid" || true
        wait "$compositor_pid" || true
    fi
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM

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
lscpu >"$report/cpu.txt"
if command -v lspci >/dev/null; then
    lspci -nnk -d ::0300 >"$report/gpu.txt"
fi
rustc --version >"$report/rust-version.txt"
ps -o pid=,args= -p "$SERVER_PID" >"$report/server-process.txt"
if [[ -n ${WM_PID:-} ]]; then
    ps -o pid=,args= -p "$WM_PID" >"$report/wm-process.txt"
fi
xdpyinfo >"$report/server.txt"
xrandr --listproviders >"$report/providers.txt"
outputs >"$report/outputs.txt"
target/release/compust --diagnose >"$report/diagnose.txt"
cp "$config" "$report/compust.toml"
sha256sum target/release/compust target/release/examples/desktop_probe >"$report/binaries.txt"
date -Is >"$report/started.txt"

target/release/compust --display "$DISPLAY" --config "$report/compust.toml" \
    >"$report/compust.log" 2>&1 &
compositor_pid=$!
target/release/examples/desktop_probe --display "$DISPLAY" --hotplug \
    --process "compust:$compositor_pid" "${measured[@]}" \
    --seconds "${SECONDS_PER_PHASE:-10}" --output "$report" --presentation "$presentation" \
    2>&1 | tee "$report/probe.log"
kill -TERM "$compositor_pid"
wait "$compositor_pid"
compositor_pid=
printf 'Compositor exited successfully after SIGTERM.\n' >"$report/shutdown.txt"
outputs >"$report/outputs-final.txt"
printf 'Monitor checks passed: %s\n' "$report"
