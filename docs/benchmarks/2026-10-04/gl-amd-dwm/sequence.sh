#!/usr/bin/env bash
# Compust's XRender and GPU renderers side by side: a snapshot of a fixed blurred scene under
# each, then every benchmark scene under both, twice per monitor layout.
set -uo pipefail
cd "$(git rev-parse --show-toplevel)"
out=$(realpath -m "${REPORTS:-artifacts}/bench-gl")
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
export SERVER_PID WM_PID SECONDS_PER_PHASE=20 COMPOSITORS="compust compust-gl"
SERVER_PID=$(pgrep -x Xorg)
WM_PID=$(pgrep -x dwm)
xrandr --output HDMI-1 --auto --right-of DP-2
sleep 3
xset dpms force on s reset
for config in compust-blur compust-gl-blur; do
    target/release/compust --config "tools/bench/$config.toml" >"$out/$config.log" 2>&1 &
    pid=$!
    sleep 2
    target/release/examples/desktop_probe --display "$DISPLAY" --snapshot "$out/$config.ppm" \
        --process "compositor:$pid" --output "$out" >"$out/$config-probe.log" 2>&1
    echo "snapshot $config exit=$?"
    kill -TERM "$pid"
    wait "$pid"
done
python3 - "$out/compust-blur.ppm" "$out/compust-gl-blur.ppm" >"$out/snapshots.txt" <<'PYTHON'
import sys
def read(path):
    magic, size, _, pixels = open(path, 'rb').read().split(b'\n', 3)
    assert magic == b'P6'
    return tuple(map(int, size.split())), pixels
(size, xrender), (other, gpu) = read(sys.argv[1]), read(sys.argv[2])
assert size == other, 'the snapshots differ in size'
worst = [max(abs(a - b) for a, b in zip(xrender[i:i + 3], gpu[i:i + 3])) for i in range(0, len(gpu), 3)]
print(f'{size[0]}x{size[1]} pixels; largest channel difference {max(worst)}')
for limit in (0, 1, 2, 4, 8, 16):
    print(f'pixels differing by more than {limit}: {sum(d > limit for d in worst)}')
PYTHON
cat "$out/snapshots.txt"
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
