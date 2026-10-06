"""Live-session checks of compust under qtile. Usage: driver.py OUT_DIR BACKEND_LABEL

Runs on an empty qtile group, measures the composed screen, and returns to the starting group.
"""
import io
import json
import math
import os
import re
import subprocess
import sys
import time

import numpy as np
from PIL import Image

HERE = os.path.dirname(os.path.abspath(__file__))
QPY = os.path.expanduser("~/.local/share/uv/tools/qtile/bin/python")
OUT, LABEL = sys.argv[1], sys.argv[2]
GROUP = "7"
RADIUS = 8
os.makedirs(OUT, exist_ok=True)
results = []
clients = []


def sh(*cmd):
    return subprocess.run(cmd, capture_output=True, text=True).stdout


def q(*args):
    return sh("qtile", "cmd-obj", *args)


def capture(name):
    data = subprocess.run(["import", "-window", "root", "-depth", "8", "png:-"], capture_output=True).stdout
    image = Image.open(io.BytesIO(data)).convert("RGB")
    image.save(f"{OUT}/{name}.png")
    return np.asarray(image).astype(int)


def geometry(wid):
    """Outer frame rectangle (x, y, w, h) including the X border, and the border width."""
    text = sh("xwininfo", "-id", str(wid))
    get = lambda key: int(re.search(key + r":\s+(-?\d+)", text).group(1))
    b = get("Border width")
    return get("Absolute upper-left X"), get("Absolute upper-left Y"), get("Width") + 2 * b, get("Height") + 2 * b, b


def active():
    return int(re.search(r"# (0x[0-9a-f]+)", sh("xprop", "-root", "_NET_ACTIVE_WINDOW")).group(1), 16)


def show(specs):
    proc = subprocess.Popen([QPY, f"{HERE}/client.py", "show", json.dumps(specs)], stdout=subprocess.PIPE, text=True)
    clients.append(proc)
    wids = []
    for line in proc.stdout:
        if line.strip() == "ready":
            break
        wids.append(int(line))
    time.sleep(0.9)
    return wids, proc


def record(name, ok, detail):
    results.append({"check": name, "ok": bool(ok), "detail": detail})
    print(("PASS " if ok else "FAIL ") + name + " — " + detail, flush=True)


def near(a, b, tol=3):
    return np.all(np.abs(np.asarray(a) - np.asarray(b)) <= tol, axis=-1)


def corners(img, ref, geo, fill, label, shadowed=True, left_only=False):
    """Check the four rounded corners: border ring in the border color, interior in `fill`,
    and nothing brighter than the background outside the outer arc."""
    x, y, w, h, b = geo
    # The border color at the middle of the top edge, or of the left edge.
    edge = img[y + h // 2, x + b // 2] if left_only else img[y + b // 2, x + w // 2]
    wrong_out = wrong_ring = wrong_in = n_out = n_ring = n_in = 0
    for cx, cy, sx, sy in ((x + RADIUS, y + RADIUS, -1, -1), (x + w - RADIUS, y + RADIUS, 1, -1),
                           (x + RADIUS, y + h - RADIUS, -1, 1), (x + w - RADIUS, y + h - RADIUS, 1, 1)):
        if left_only and sx > 0:
            continue
        for i in range(RADIUS):
            for j in range(RADIUS):
                px = cx + (i if sx > 0 else -i - 1)
                py = cy + (j if sy > 0 else -j - 1)
                d = math.hypot(i + 0.5, j + 0.5)
                p = img[py, px]
                if d > RADIUS + 0.8:
                    n_out += 1
                    # Outside: the background, possibly darkened by a shadow; never window or border color.
                    if np.any(p > ref[py, px] + 3) or (not shadowed and not near(p, ref[py, px])):
                        wrong_out += 1
                elif RADIUS - b + 0.8 < d < RADIUS - 0.8:
                    n_ring += 1
                    wrong_ring += not near(p, edge)
                elif d < RADIUS - b - 0.8:
                    n_in += 1
                    if fill is not None:
                        wrong_in += not near(p, fill)
    ok = wrong_out == 0 and wrong_ring == 0 and wrong_in == 0
    record(f"corners: {label}", ok,
           f"border {tuple(int(v) for v in edge)}, width {b}; wrong pixels outside/ring/inside = "
           f"{wrong_out}/{n_out}, {wrong_ring}/{n_ring}, {wrong_in}/{n_in}")
    return tuple(int(v) for v in edge)


def ticks(pid):
    fields = open(f"/proc/{pid}/stat").read().rsplit(")", 1)[1].split()
    return int(fields[11]) + int(fields[12])


def rss(pid):
    return int(re.search(r"VmRSS:\s+(\d+)", open(f"/proc/{pid}/status").read()).group(1))


def measure(name, seconds, pids):
    hz = os.sysconf("SC_CLK_TCK")
    before = {k: ticks(p) for k, p in pids.items()}
    time.sleep(seconds)
    cpu = {k: round(100 * (ticks(p) - before[k]) / hz / seconds, 1) for k, p in pids.items()}
    row = {"phase": name, "seconds": seconds, "cpu_percent": cpu, "compust_rss_kib": rss(pids["compust"]),
           "xorg_rss_kib": rss(pids["xorg"])}
    print("MEASURE", json.dumps(row), flush=True)
    return row


pids = {"compust": int(sh("pgrep", "-x", "compust").split()[0]), "xorg": int(sh("pgrep", "-x", "Xorg").split()[0])}
start_group = json.loads(q("-o", "group", "-f", "info"))["name"]
measurements = []
try:
    q("-o", "group", GROUP, "-f", "toscreen")
    q("-o", "group", GROUP, "-f", "setlayout", "-a", "tall")
    time.sleep(1.0)
    ref = capture("0-empty")
    rss_start = rss(pids["compust"])

    # Scene 1: two tiled opaque windows with qtile's X borders.
    (white, checker), tiled = show([
        {"name": "white", "color": [255, 255, 255]},
        {"name": "checker", "color": "checker"},
    ])
    q("-o", "window", str(checker), "-f", "focus")
    time.sleep(0.6)
    img = capture("1-tiled")
    gw, gc = geometry(white), geometry(checker)
    record("tiling: two windows side by side with borders", gw[4] > 0 and gc[4] > 0 and gw[0] + gw[2] <= gc[0],
           f"white {gw}, checker {gc}, active {active() == checker}")
    unfocused = corners(img, ref, gw, (255, 255, 255), "tiled white, unfocused")
    focused = corners(img, ref, gc, None, "tiled checker, focused")
    record("borders: focused and unfocused colors differ", focused != unfocused, f"focused {focused}, unfocused {unfocused}")

    # Shadow of the white window onto the wallpaper, below its bottom edge at mid-width.
    x, y, w, h, b = gw
    column = x + w // 2
    rows = slice(y + h // 2 - 100, y + h // 2 + 100)
    gap = gc[0] - (x + w)
    alpha = [round(float(1 - img[rows, x + w + d].sum() / max(ref[rows, x + w + d].sum(), 1)), 2) for d in range(gap)]
    monotone = all(alpha[i] >= alpha[i + 1] - 0.03 for i in range(len(alpha) - 1))
    record("shadow: right of a tiled window, in the gap over the wallpaper", alpha[0] > 0.5 and monotone and alpha[-1] > 0.02,
           f"darkening across the {gap} px gap: {alpha}; wallpaper brightness there {ref[rows, x + w:x + w + gap].mean():.0f}/255")
    above = [round(float(1 - img[y - d, column].sum() / max(ref[y - d, column].sum(), 1)), 2) for d in range(1, 9)]
    record("shadow: above the window, limited by the offset", above[-1] <= 0.02 and above[-2] <= 0.02,
           f"darkening 1..8 px above: {above} (radius 10 - offset 5 = 5)")

    # Scene 2: focus moves; the rounded border must follow the new colors.
    q("-o", "window", str(white), "-f", "focus")
    time.sleep(0.6)
    img2 = capture("2-focus-moved")
    now_focused = corners(img2, ref, gw, (255, 255, 255), "tiled white, after gaining focus")
    now_unfocused = corners(img2, ref, gc, None, "tiled checker, after losing focus")
    record("borders: colors swap when focus moves", now_focused == focused and now_unfocused == unfocused,
           f"white {now_focused}, checker {now_unfocused}")

    # Scene 3: floating windows above the tiles: half-transparent red and opaque blue.
    (red, blue), floats = show([
        {"name": "red-50", "color": [255, 0, 0], "w": 420, "h": 260, "float": True, "opacity": 50},
        {"name": "blue", "color": [0, 0, 255], "w": 260, "h": 180, "float": True},
    ])
    q("-o", "window", str(red), "-f", "set_position_floating", "-a", str(gc[0] - 210), str(gw[1] + 120))
    q("-o", "window", str(blue), "-f", "set_position_floating", "-a", str(gw[0] + 80), str(gw[1] + 420))
    q("-o", "window", str(blue), "-f", "focus")
    time.sleep(0.9)
    img3 = capture("3-floating")
    gr, gb = geometry(red), geometry(blue)
    floating = gr[2] == 420 + 2 * gr[4] and gb[2] == 260 + 2 * gb[4]
    record("floating: dialogs keep their size above the tiles", floating, f"red {gr}, blue {gb}")
    rx, ry, rw, rh, rb = gr
    # The part of the red window over the white tile, and the part over the checkered tile.
    over_white = img3[ry + 40:ry + rh - 40, rx + 20:gw[0] + gw[2] - 25]
    over_check = img3[ry + 40:ry + rh - 40, gc[0] + 25:rx + rw - 20]
    mean_w = over_white.reshape(-1, 3).mean(0).round(1)
    record("transparency: 50% red over white", near(mean_w, (255, 128, 128)) and over_white.std(axis=(0, 1)).max() < 1.5,
           f"mean {mean_w.tolist()}, expected (255, 128, 128); {over_white.shape[1]}x{over_white.shape[0]} px")
    mean_c = over_check.reshape(-1, 3).mean(0).round(1)
    spread = float(over_check[..., 1].std())
    low, high = int(over_check[..., 1].min()), int(over_check[..., 1].max())
    green = over_check[..., 1]
    weighted = bool(np.all((np.abs(green - 32) <= 3) | (np.abs(green - 96) <= 3)))
    record("blur: 2 px checker behind the 50% red window", weighted and near(mean_c, (191, 64, 64), 3),
           f"mean {mean_c.tolist()}, green from {low} to {high}; the documented weighting (backdrop shown at the window's 50%) "
           f"gives 32 and 96 around a flat 64, and no blur would give 0 and 128")
    edge = corners(img3, img2, gr, (255, 128, 128), "floating 50% red, left corners over white", left_only=True)
    record("transparency: its border is 50% of qtile's border color over white", near(edge, [(v + 255) / 2 for v in unfocused], 3),
           f"border {edge}, expected {[round((v + 255) / 2) for v in unfocused]}")
    bx, by, bw, bh, bb = gb
    row = by + bh // 2
    right = [round(float(1 - img3[row, bx + bw + d].sum() / 765), 2) for d in range(0, 18)]
    reach = next((d for d, a in enumerate(right) if a <= 0.01), None)
    record("shadow: right of the opaque floating window, on white", 0.6 < right[0] <= 0.92 and right[5] <= 0.45 <= right[4] and reach in range(12, 17),
           f"darkness by distance 0..17 px: {right}; gone at {reach} px; half of the 0.90 maximum falls 5 px out, at the offset")
    left = [round(float(1 - img3[row, bx - d].sum() / 765), 2) for d in range(1, 9)]
    record("shadow: left of it, limited by the offset", left[5] <= 0.01, f"darkness 1..8 px left: {left}")

    # Scene 4: leave the group and come back; the frame must be the same.
    q("-o", "group", start_group, "-f", "toscreen")
    time.sleep(0.8)
    q("-o", "group", GROUP, "-f", "toscreen")
    time.sleep(1.0)
    q("-o", "window", str(blue), "-f", "focus")
    time.sleep(0.6)
    img4 = capture("4-after-group-switch")
    changed = int(np.any(np.abs(img4[40:] - img3[40:]) > 2, axis=-1).sum())
    record("workspaces: same frame after switching away and back", changed == 0, f"{changed} pixels differ below the bar")

    # Measurements: idle, then a tiled window alternating red and blue behind the floats.
    measurements.append(measure("idle, 4 windows", 6, pids))
    floats.terminate()
    tiled.terminate()
    time.sleep(0.8)
    (flip, ), flipping = show([{"name": "flip", "color": [255, 0, 0], "animate": True}])
    measurements.append(measure("opaque window repainting at 60 Hz", 8, pids))
    (veil, ), veiling = show([{"name": "veil", "color": [255, 255, 255], "w": 600, "h": 400, "float": True, "opacity": 50}])
    measurements.append(measure("same, under a 600x400 50% window with blur", 8, pids))
    veiling.terminate()

    # Scene 5: fullscreen through qtile.
    q("-o", "window", str(flip), "-f", "toggle_fullscreen")
    time.sleep(0.8)
    full = capture("5-fullscreen")
    gf = geometry(flip)
    corner_pixels = [tuple(int(v) for v in full[py, px]) for px, py in ((0, 0), (full.shape[1] - 1, 0), (0, full.shape[0] - 1), (full.shape[1] - 1, full.shape[0] - 1))]
    covered = all(p[1] == 0 and (p[0] > 250 or p[2] > 250) for p in corner_pixels)
    record("fullscreen: window covers the screen, corners included", gf[:4] == (0, 0, full.shape[1], full.shape[0]) and covered,
           f"geometry {gf}, screen corner pixels {corner_pixels}")
    q("-o", "window", str(flip), "-f", "toggle_fullscreen")
    time.sleep(0.5)
    flipping.terminate()
    time.sleep(0.8)

    # Scene 6: rapid create/map/destroy, then the empty group must match the first capture.
    subprocess.run([QPY, f"{HERE}/client.py", "churn", "32"])
    time.sleep(1.2)
    end = capture("6-empty-again")
    changed = int(np.any(np.abs(end[40:] - ref[40:]) > 2, axis=-1).sum())
    record("lifecycle: empty group restored after 32 rapid windows", changed == 0, f"{changed} pixels differ from the first empty capture")
    measurements.append(measure("idle, empty group", 6, pids))
    record("process: compositor still running, same pid", sh("pgrep", "-x", "compust").split() == [str(pids["compust"])],
           f"RSS {rss_start} -> {rss(pids['compust'])} KiB")
finally:
    for proc in clients:
        if proc.poll() is None:
            proc.terminate()
    q("-o", "group", start_group, "-f", "toscreen")
    json.dump({"backend": LABEL, "results": results, "measurements": measurements}, open(f"{OUT}/results.json", "w"), indent=1)
failed = [r for r in results if not r["ok"]]
print(f"{LABEL}: {len(results) - len(failed)} passed, {len(failed)} failed")
