"""Предпросмотр .cbm: программный рендер частей (краска окрашена, отрывные - красные)."""
import json, struct, sys, os, glob
import numpy as np
from PIL import Image, ImageDraw

def read(path):
    b = open(path, "rb").read()
    hl = struct.unpack("<I", b[4:8])[0]
    h = json.loads(b[8:8 + hl]); base = 8 + hl
    parts = []
    for p in h["parts"]:
        st = 12 if b[:4] == b"CBM2" else 10
        v = np.frombuffer(b, dtype="<f4", count=p["verts"] * st, offset=base + p["offset"]).reshape(-1, st)
        parts.append((p, v))
    return parts

def render(path, cmap, d, ox, oy, sc, yaw):
    cm = np.asarray(Image.open(cmap).convert("RGB"))
    items = []
    a = np.radians(yaw); R = np.array([[np.cos(a), 0, np.sin(a)], [0, 1, 0], [-np.sin(a), 0, np.cos(a)]])
    e = np.radians(18); R = np.array([[1, 0, 0], [0, np.cos(e), -np.sin(e)], [0, np.sin(e), np.cos(e)]]) @ R
    L = np.array([0.4, 0.8, -0.45]); L /= np.linalg.norm(L)
    for p, v in parts_cache(path):
        c = np.array(p["center"])
        P = v[:, :3] + c; N = v[:, 3:6]; UV = v[:, 6:8]
        for t in range(0, len(P), 3):
            uv = UV[t:t + 3].mean(0)
            col = cm[int(np.clip(uv[1] * 511, 0, 511)), int(np.clip(uv[0] * 511, 0, 511))].astype(float)
            if p["name"] == "paint": col = np.array([230, 120, 40.0])
            if p["name"] == "detach": col = np.array([220, 30, 30.0])
            n = N[t:t + 3].mean(0); sh = 0.45 + 0.55 * max(0, float(n @ L))
            Q = (R @ P[t:t + 3].T).T
            items.append((Q[:, 2].mean(), Q, tuple(int(x) for x in col * sh)))
    items.sort(key=lambda x: x[0])
    for z, Q, rgb in items:
        d.polygon([(ox + q[0] * sc, oy - q[1] * sc) for q in Q], fill=rgb)

_cache = {}
def parts_cache(p):
    if p not in _cache: _cache[p] = read(p)
    return _cache[p]

if __name__ == "__main__":
    files = sorted(glob.glob(os.path.join(sys.argv[1], "*.cbm")))
    W, H = 360, 240
    img = Image.new("RGB", (W * 4, H * ((len(files) + 3) // 4)), (34, 36, 40)); d = ImageDraw.Draw(img)
    for i, f in enumerate(files):
        ox, oy = (i % 4) * W + W // 2, (i // 4) * H + int(H * 0.68)
        render(f, sys.argv[2], d, ox, oy, 42, 215)
        d.text((ox - W // 2 + 6, oy - int(H * 0.68) + 6), os.path.basename(f), fill=(255, 220, 90))
    img.save(sys.argv[3])
