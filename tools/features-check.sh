#!/usr/bin/env bash
set -euo pipefail

usage='Usage: tools/features-check.sh NEW_REPORT_ROOT [both|xrender]'
if [[ ${1:-} == --help ]]; then
    printf '%s\n' "$usage"
    printf 'Check shadow pixels, focus rules, and fullscreen suspension without a window manager.\n'
    printf 'Build release binaries first. Both renderers are required by default.\n'
    printf 'Use xrender for an Xvfb rehearsal; Xvfb cannot provide DRI3 for the GPU painter.\n'
    printf 'DESKTOP_DISPLAY and SERVER_PID select an existing dedicated X server.\n'
    exit 0
fi
if (( $# < 1 || $# > 2 )); then
    printf '%s\n' "$usage" >&2
    exit 2
fi
case ${2:-both} in
    both) backends=(xrender gl) ;;
    xrender) backends=(xrender) ;;
    *) printf 'Unknown renderer selection: %s\n' "$2" >&2; exit 2 ;;
esac
if [[ -n ${DESKTOP_DISPLAY:-} && -z ${SERVER_PID:-} ]]; then
    printf 'SERVER_PID must identify the X server of DESKTOP_DISPLAY.\n' >&2
    exit 2
fi
root=$(realpath -m -- "$1")
cd -- "$(dirname -- "$(realpath -- "$0")")/.."
mkdir -- "$root"
work=$(mktemp -d /tmp/compust-features.XXXXXX)
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

if [[ -n ${DESKTOP_DISPLAY:-} ]]; then
    export DISPLAY=$DESKTOP_DISPLAY
    server_pid=$SERVER_PID
    servers=()
else
    mkfifo "$work/display"
    exec {server_ready}<>"$work/display"
    "${XVFB:-Xvfb}" -displayfd 3 -screen 0 1280x800x24 -nolisten tcp -noreset \
        3>&"$server_ready" >"$root/xvfb.log" 2>&1 &
    server_pid=$!
    pids=("$server_pid")
    IFS= read -r -t 10 -u "$server_ready" server_display
    exec {server_ready}>&-
    export DISPLAY=":$server_display"
    servers=("$(command -v "${XVFB:-Xvfb}")")
fi
xdpyinfo >"$root/server.txt"
xrandr --current >"$root/outputs.txt"
xrandr --listproviders >"$root/providers.txt"
ps -o pid=,args= -p "$server_pid" >"$root/server-process.txt"
sources=(src crates examples tools Cargo.toml Cargo.lock rust-toolchain.toml)
git rev-parse HEAD >"$root/commit.txt"
git status --short >"$root/worktree.txt"
{
    git diff HEAD -- "${sources[@]}"
    while IFS= read -r -d '' file; do
        git diff --no-index -- /dev/null "$file" || [[ $? == 1 ]]
    done < <(git ls-files -z --others --exclude-standard -- "${sources[@]}")
} >"$root/source.patch"
git ls-files -z --cached --others --exclude-standard -- "${sources[@]}" \
    | xargs -0 sha256sum >"$root/source-sha256.txt"
sha256sum target/release/compust target/release/examples/desktop_probe "${servers[@]}" >"$root/binaries.txt"
uname -srmo >"$root/kernel.txt"
date -Is >"$root/started.txt"
lscpu >"$root/cpu.txt"
rustc --version >"$root/rust-version.txt"

for backend in "${backends[@]}"; do
    report="$root/$backend"
    mkdir -- "$report"
    {
        printf 'backend = "%s"\n' "$backend"
        cat tools/desktop/compust-features.toml
    } >"$report/compust.toml"
    RUST_LOG=compust=info target/release/compust --display "$DISPLAY" --config "$report/compust.toml" \
        >"$report/compust.log" 2>&1 &
    compositor_pid=$!
    pids=("$compositor_pid" "${pids[@]}")
    reference=()
    if [[ $backend == gl ]]; then
        reference=(--reference "$root/xrender")
    fi
    if timeout 180s target/release/examples/desktop_probe --display "$DISPLAY" --features \
        --process "compust:$compositor_pid" --process "server:$server_pid" \
        --seconds "${SECONDS_PER_PHASE:-10}" --output "$report" "${reference[@]}" \
        >"$report/probe.log" 2>&1; then
        if [[ $backend == gl ]] && { ! grep -Fq 'drawing with the GPU' "$report/compust.log" \
            || grep -Eq 'drawing with XRender|continuing with XRender' "$report/compust.log"; }; then
            printf 'GPU check failed: Compust used XRender. See %s/compust.log\n' "$report" >&2
            exit 1
        fi
    else
        printf 'Feature checks failed: %s\n' "$report" >&2
        tail -n 20 "$report/probe.log" >&2
        exit 1
    fi
    kill -TERM "$compositor_pid"
    wait "$compositor_pid"
    pids=("${pids[@]:1}")
    printf 'Compositor exited successfully after SIGTERM.\n' >"$report/shutdown.txt"
    printf '%s: feature checks passed\n' "$backend"
done
date -Is >"$root/finished.txt"
printf 'Feature checks passed: %s\n' "$root"
