#!/usr/bin/env bash
set -euo pipefail

usage() {
    printf 'Usage: SERVER_PID=PID [WM_PID=PID] tools/bench.sh NEW_REPORT_DIRECTORY\n'
}
if [[ ${1:-} == --help ]]; then
    usage
    printf 'Measure the benchmark scenes on DISPLAY under each compositor in turn. Stop the\n'
    printf "display's compositor first and build release binaries, including the probe.\n"
    printf 'COMPOSITORS defaults to "compust picom-xrender picom-glx"; SCENES to every scene,\n'
    printf 'where a ":blur" suffix selects the blur configurations. SECONDS_PER_PHASE: 1-30.\n'
    printf 'GPU_BUSY names a GPU load file to sample; the first amdgpu one is used by default.\n'
    exit 0
fi
if (( $# != 1 )); then
    usage >&2
    exit 2
fi
if [[ -z ${DISPLAY:-} || -z ${SERVER_PID:-} ]]; then
    usage >&2
    printf 'DISPLAY and SERVER_PID must identify the X server to measure.\n' >&2
    exit 2
fi
read -r -a compositors <<<"${COMPOSITORS:-compust picom-xrender picom-glx}"
read -r -a scenes <<<"${SCENES:-idle small-update fullscreen-translucent \
fullscreen-translucent:blur eight-translucent eight-translucent:blur covered covered:blur \
move-resize open-close}"
for compositor in "${compositors[@]}"; do
    case "$compositor" in
        compust | picom-xrender | picom-glx) ;;
        *) printf 'Unknown compositor: %s\n' "$compositor" >&2; exit 2 ;;
    esac
done
report=$1
mkdir -- "$report"
report=$(realpath "$report")
measured=(--process "server:$SERVER_PID")
if [[ -n ${WM_PID:-} ]]; then
    measured+=(--process "wm:$WM_PID")
fi
gpu_busy=${GPU_BUSY:-$(compgen -G '/sys/class/drm/card*/device/gpu_busy_percent' | head -n 1 || true)}
if [[ -n $gpu_busy ]]; then
    measured+=(--gpu-busy "$gpu_busy")
fi
# Present stops completing frames while DPMS has the monitors off, so keep them on and
# restore the screen saver and DPMS settings afterward.
xset q >"$report/xset.txt"
read -r saver_timeout saver_cycle < <(awk '/timeout:/ { print $2, $4; exit }' "$report/xset.txt")
read -r standby suspend off < <(awk '/Standby:/ { print $2, $4, $6; exit }' "$report/xset.txt")
compositor_pid=
cleanup() {
    if [[ -n $compositor_pid ]] && kill -0 "$compositor_pid" 2>/dev/null; then
        kill -TERM "$compositor_pid" || true
        wait "$compositor_pid" || true
    fi
    xset s "$saver_timeout" "$saver_cycle" dpms "$standby" "$suspend" "$off"
    if grep -q 'DPMS is Disabled' "$report/xset.txt"; then
        xset -dpms
    fi
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM

xset dpms force on s reset
xset s off -dpms

sources=(src examples tools Cargo.toml Cargo.lock rust-toolchain.toml)
git rev-parse HEAD >"$report/commit.txt"
git status --short >"$report/worktree.txt"
{
    git diff HEAD -- "${sources[@]}"
    while IFS= read -r -d '' file; do
        git diff --no-index -- /dev/null "$file" || [[ $? == 1 ]]
    done < <(git ls-files -z --others --exclude-standard -- "${sources[@]}")
} >"$report/source.patch"
uname -srmo >"$report/kernel.txt"
lscpu >"$report/cpu.txt"
if command -v lspci >/dev/null; then
    lspci -nnk -d ::0300 >"$report/gpu.txt"
fi
printf '%s\n' "${gpu_busy:-none}" >"$report/gpu-busy.txt"
if command -v glxinfo >/dev/null; then
    glxinfo -B >"$report/gl.txt" 2>&1 || true
fi
rustc --version >"$report/rust-version.txt"
ps -o pid=,args= -p "$SERVER_PID" >"$report/server-process.txt"
if [[ -n ${WM_PID:-} ]]; then
    ps -o pid=,args= -p "$WM_PID" >"$report/wm-process.txt"
fi
xdpyinfo >"$report/server.txt"
xrandr --listproviders >"$report/providers.txt"
# Monitor EDIDs contain serial numbers; keep the other output properties.
xrandr --verbose | awk '/^\t[^\t]/ { edid = /^\tEDID:/ } !(edid && /^\t\t/)' >"$report/outputs.txt"
target/release/compust --diagnose >"$report/diagnose.txt"
mkdir "$report/configs"
cp tools/bench/*.toml tools/bench/*.conf "$report/configs/"
binaries=(target/release/compust target/release/examples/desktop_probe)
if [[ " ${compositors[*]} " == *" picom-"* ]]; then
    picom --version >"$report/picom-version.txt"
    binaries+=("$(command -v picom)")
fi
sha256sum "${binaries[@]}" >"$report/binaries.txt"
date -Is >"$report/started.txt"

start() {
    local compositor=$1 blur=$2 log=$3 suffix=
    [[ $blur == 1 ]] && suffix=-blur
    case "$compositor" in
        compust)
            target/release/compust --display "$DISPLAY" \
                --config "$report/configs/compust$suffix.toml" >"$log" 2>&1 &
            ;;
        picom-*)
            local backend=${compositor#picom-} config=picom.conf
            [[ $blur == 1 ]] && config=picom-blur-$backend.conf
            picom --config "$report/configs/$config" --backend "$backend" >"$log" 2>&1 &
            ;;
    esac
    compositor_pid=$!
}

status=0
summary=$report/summary.csv
for compositor in "${compositors[@]}"; do
    for spec in "${scenes[@]}"; do
        scene=${spec%:blur}
        blur=0
        [[ $spec == *:blur ]] && blur=1
        name=$scene
        (( blur )) && name=$scene-blur
        output=$report/$compositor/$name
        mkdir -p "$output"
        start "$compositor" "$blur" "$output/compositor.log"
        if timeout 120s target/release/examples/desktop_probe --display "$DISPLAY" \
            --bench "$scene" --process "compositor:$compositor_pid" "${measured[@]}" \
            --seconds "${SECONDS_PER_PHASE:-10}" --output "$output" \
            >"$output/probe.log" 2>&1 && kill -0 "$compositor_pid" 2>/dev/null; then
            if [[ ! -e $summary ]]; then
                { printf 'compositor,blur,'; head -n 1 "$output/summary.csv"; } >"$summary"
            fi
            { printf '%s,%s,' "$compositor" "$blur"; tail -n 1 "$output/summary.csv"; } >>"$summary"
            printf '%s %s: %s\n' "$compositor" "$name" "$(tail -n 1 "$output/probe.log")"
        else
            printf '%s %s: failed\n' "$compositor" "$name"
            tail -n 3 "$output/probe.log" || true
            status=1
        fi
        if kill -0 "$compositor_pid" 2>/dev/null; then
            kill -TERM "$compositor_pid"
            # picom does not handle SIGTERM, so 143 is its normal end here.
            exit_status=0
            wait "$compositor_pid" || exit_status=$?
            if (( exit_status != 0 && exit_status != 143 )); then
                printf '%s exited with status %s\n' "$compositor" "$exit_status"
            fi
        else
            wait "$compositor_pid" || true
            printf '%s exited during %s\n' "$compositor" "$name"
            status=1
        fi
        compositor_pid=
    done
done
date -Is >"$report/finished.txt"
printf 'Benchmark report: %s\n' "$report"
exit "$status"
