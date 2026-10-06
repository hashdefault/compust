"""Synthetic X11 test windows. Run with qtile's Python (xcffib).

client.py show '<json list of window specs>'   prints one window id per line, then "ready"
client.py churn N                                 creates, maps and destroys N windows
A spec: {"name", "color": [r,g,b] | "checker", "w", "h", "float": bool, "opacity": 0..100,
         "animate": bool}
"""
import json
import struct
import sys
import time

import xcffib
import xcffib.xproto as xp

conn = xcffib.connect()
setup = conn.get_setup()
screen = setup.roots[0]
core = conn.core


def atom(name):
    return core.InternAtom(False, len(name), name).reply().atom


def pixel(rgb):
    return (rgb[0] << 16) | (rgb[1] << 8) | rgb[2]


def make(spec):
    wid = conn.generate_id()
    mask = xp.CW.BackPixel | xp.CW.EventMask
    values = [0xFFFFFF, xp.EventMask.StructureNotify]
    if spec["color"] == "checker":
        pix = conn.generate_id()
        c = spec.get("cell", 2)
        core.CreatePixmap(screen.root_depth, pix, screen.root, 2 * c, 2 * c)
        gc = conn.generate_id()
        core.CreateGC(gc, pix, xp.GC.Foreground, [0xFFFFFF])
        core.PolyFillRectangle(pix, gc, 1, [xp.RECTANGLE.synthetic(0, 0, 2 * c, 2 * c)])
        core.ChangeGC(gc, xp.GC.Foreground, [0x000000])
        core.PolyFillRectangle(
            pix, gc, 2,
            [xp.RECTANGLE.synthetic(0, 0, c, c), xp.RECTANGLE.synthetic(c, c, c, c)],
        )
        mask = xp.CW.BackPixmap | xp.CW.EventMask
        values = [pix, xp.EventMask.StructureNotify]
    else:
        values[0] = pixel(spec["color"])
    core.CreateWindow(
        screen.root_depth, wid, screen.root, 0, 0, spec.get("w", 400), spec.get("h", 300), 0,
        xp.WindowClass.InputOutput, screen.root_visual, mask, values,
    )
    name = spec["name"].encode()
    core.ChangeProperty(xp.PropMode.Replace, wid, xp.Atom.WM_NAME, xp.Atom.STRING, 8, len(name), name)
    cls = b"compust-test\0Compust-test\0"
    core.ChangeProperty(xp.PropMode.Replace, wid, xp.Atom.WM_CLASS, xp.Atom.STRING, 8, len(cls), cls)
    if spec.get("float"):
        data = struct.pack("I", atom("_NET_WM_WINDOW_TYPE_DIALOG"))
        core.ChangeProperty(
            xp.PropMode.Replace, wid, atom("_NET_WM_WINDOW_TYPE"), xp.Atom.ATOM, 32, 1, data
        )
    if "opacity" in spec:
        data = struct.pack("I", spec["opacity"] * 0xFFFFFFFF // 100)
        core.ChangeProperty(
            xp.PropMode.Replace, wid, atom("_NET_WM_WINDOW_OPACITY"), xp.Atom.CARDINAL, 32, 1, data
        )
    core.MapWindow(wid)
    return wid


if sys.argv[1] == "churn":
    for i in range(int(sys.argv[2])):
        wid = make({"name": f"churn-{i}", "color": [0, 200, 0], "w": 300, "h": 200})
        conn.flush()
        core.DestroyWindow(wid)
        conn.flush()
    conn.get_setup()
    core.GetInputFocus().reply()
    sys.exit(0)

specs = json.loads(sys.argv[2])
wids = []
for spec in specs:
    wids.append(make(spec))
    conn.flush()
    time.sleep(0.25)
for wid in wids:
    print(wid, flush=True)
print("ready", flush=True)

animated = [w for w, s in zip(wids, specs) if s.get("animate")]
frame = 0
while True:
    if animated:
        frame += 1
        for wid in animated:
            core.ChangeWindowAttributes(wid, xp.CW.BackPixel, [0xFF0000 if frame % 2 else 0x0000FF])
            core.ClearArea(False, wid, 0, 0, 0, 0)
        conn.flush()
        time.sleep(1 / 60)
    else:
        time.sleep(0.5)
    while conn.poll_for_event() is not None:
        pass
