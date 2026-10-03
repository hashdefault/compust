#!/usr/bin/env bash
# Live reload check: one Compust process, three probe runs
# (Present, direct, Present) with monitor transitions and SIGUSR1 reloads between samples.
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"
R=$1
mkdir -p "$R"
cfg=$R/compust.toml
server_pid=$(pgrep -x Xorg | head -1)
wm_pid=$(pgrep -x dwm | head -1)

outputs() { xrandr --verbose | awk '/^\t[^\t]/ { edid = /^\tEDID:/ } !(edid && /^\t\t/)'; }
restore() {
    xrandr --output HDMI-1 --right-of DP-2 2>/dev/null || true
    xrandr --output HDMI-1 --off --output DP-2 --pos 0x0 || true
}
compositor_pid=
probe_pid=
cleanup() {
    [[ -n $probe_pid ]] && kill "$probe_pid" 2>/dev/null || true
    exec 3>&- 2>/dev/null || true
    if [[ -n $compositor_pid ]] && kill -0 "$compositor_pid" 2>/dev/null; then
        kill -TERM "$compositor_pid" || true
        wait "$compositor_pid" || true
    fi
    restore
    outputs >"$R/outputs-final.txt"
}
trap cleanup EXIT

git rev-parse HEAD >"$R/commit.txt"
git diff HEAD -- src examples tools tests Cargo.toml Cargo.lock >"$R/source.patch"
git ls-files --others --exclude-standard -- src tests | xargs -r sha256sum >"$R/untracked-sha256.txt"
uname -srmo >"$R/kernel.txt"
lscpu | grep -E "Model name|^CPU\(s\)" >"$R/cpu.txt"
lspci -nnk -d ::0300 >"$R/gpu.txt"
/usr/lib/Xorg -version >"$R/server-version.txt" 2>&1 || true
pacman -Q xlibre-xserver mesa linux dwm 2>/dev/null >"$R/packages.txt" || true
outputs >"$R/outputs-initial.txt"
target/release/compust --diagnose >"$R/diagnose.txt"
sha256sum target/release/compust target/release/examples/desktop_probe >"$R/binaries.txt"
date -Is >"$R/started.txt"

reload() {
    printf '%b' "$1" >"$cfg"
    local before
    before=$(grep -c "configuration reloaded\|keeping the current configuration" "$R/compust.log" || true)
    kill -USR1 "$compositor_pid"
    for _ in $(seq 100); do
        local now
        now=$(grep -c "configuration reloaded\|keeping the current configuration" "$R/compust.log" || true)
        (( now > before )) && { echo "reload: $(tail -1 "$R/compust.log")"; return 0; }
        sleep 0.05
    done
    echo "reload not logged" >&2
    return 1
}

monitors() {
    xrandr "$@"
    sleep 2
    echo "xrandr $*: $(xrandr --listmonitors | head -1)"
}

# One probe run: $1 = mode, $2 = log name; then lines of "sample LABEL" or shell commands.
start_probe() {
    local mode=$1 name=$2
    mkdir -p "$R/$name"
    rm -f "$R/$name.in"
    mkfifo "$R/$name.in"
    target/release/examples/desktop_probe --display "$DISPLAY" --hotplug \
        --process "compust:$compositor_pid" --process "server:$server_pid" --process "wm:$wm_pid" \
        --seconds 3 --output "$R/$name" --presentation "$mode" \
        <"$R/$name.in" >"$R/$name.log" 2>&1 &
    probe_pid=$!
    exec 3>"$R/$name.in"
    wait_line "$R/$name.log" "READY"
    count=0
    log=$R/$name.log
}
wait_line() {
    for _ in $(seq 900); do
        grep -q -- "$2" "$1" && return 0
        if ! kill -0 "$probe_pid" 2>/dev/null; then
            echo "probe exited:"; tail -20 "$1"; return 1
        fi
        sleep 0.1
    done
    echo "timed out waiting for $2"; tail -20 "$1"; return 1
}
sample() {
    count=$((count + 1))
    echo "sample $1" >&3
    wait_line "$log" "PASS sample=$count label=$1 "
    grep "PASS sample=$count " "$log"
}
stop_probe() {
    echo quit >&3
    exec 3>&-
    wait "$probe_pid"
    probe_pid=
}

printf 'fade_ms = 0\nblur_radius = 0\nvsync = true\n' >"$cfg"
target/release/compust --config "$cfg" >"$R/compust.log" 2>&1 &
compositor_pid=$!
sleep 1

echo "== Present run"
start_probe present present
sample single-start
monitors --output HDMI-1 --mode 1920x1080 --rate 60 --right-of DP-2
sample dual-right
reload 'fade_ms = 120\nblur_radius = 4\nvsync = true\n'
sample dual-blur4
reload 'fade_ms = 120\nblur_radius = 4\nvsync = true\nopacity = 101\n'
sample dual-invalid
monitors --output HDMI-1 --left-of DP-2
sample dual-left
reload 'fade_ms = 0\nblur_radius = 16\nvsync = true\n'
sample dual-left-blur16
reload 'fade_ms = 0\nblur_radius = 0\nvsync = true\n'
monitors --output HDMI-1 --right-of DP-2
sample dual-right-blur0
stop_probe

echo "== direct run after reloading vsync = false"
reload 'fade_ms = 0\nblur_radius = 0\nvsync = false\n'
start_probe direct direct
sample dual-direct
reload 'fade_ms = 0\nblur_radius = 4\nvsync = false\n'
sample dual-direct-blur4
monitors --output HDMI-1 --off
sample single-direct
stop_probe

echo "== Present run after reloading vsync = true"
reload 'fade_ms = 0\nblur_radius = 0\nvsync = true\n'
start_probe present present-again
sample single-present-again
monitors --output HDMI-1 --mode 1920x1080 --rate 60 --right-of DP-2
sample dual-present-again
stop_probe

kill -TERM "$compositor_pid"
wait "$compositor_pid"
compositor_pid=
echo "compositor exited successfully after SIGTERM" | tee "$R/shutdown.txt"
date -Is >"$R/finished.txt"
echo "ALL PASSED"
