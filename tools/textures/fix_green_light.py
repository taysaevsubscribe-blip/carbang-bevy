"""Зелёный сигнал светофора старта: в исходнике он жёлто-зелёный. Делаем ярко-зелёным,
остальные лампы приглушаем. Запуск после prepare_fx.py: python3 tools/textures/fix_green_light.py <исходник png>"""
import os, sys
import numpy as np
from PIL import Image
from scipy import ndimage
sys.path.insert(0, os.path.dirname(__file__))
from prepare_fx import keyed
ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
p = os.path.join(ROOT, "assets", "ui", "pop", "start_light_green.png")
keyed(sys.argv[1], 512).save(p)
a = np.asarray(Image.open(p).convert("RGBA")).astype(np.float64) / 255
rgb, al = a[..., :3].copy(), a[..., 3]
mx, mn = rgb.max(-1), rgb.min(-1)
d = np.maximum(mx - mn, 1e-6)
r, g, b = rgb[..., 0], rgb[..., 1], rgb[..., 2]
hue = np.where(mx == r, ((g - b) / d) % 6, np.where(mx == g, (b - r) / d + 2, (r - g) / d + 4)) * 60
s = (mx - mn) / np.maximum(mx, 1e-6)
lab, n = ndimage.label((hue > 40) & (hue < 110) & (mx > 0.3) & (s > 0.25) & (al > 0.5))
best = max(((i, np.where(lab == i)[0].mean()) for i in range(1, n + 1) if (lab == i).sum() > 300), key=lambda t: t[1])[0]
lamp = ndimage.binary_fill_holes(ndimage.binary_closing(lab == best, iterations=3))
ys, xs = np.where(lamp)
cy, cx = ys.mean(), xs.mean()
rad = np.sqrt(lamp.sum() / np.pi)
yy, xx = np.mgrid[0:a.shape[0], 0:a.shape[1]]
dist = np.hypot(yy - cy + rad * 0.25, xx - cx + rad * 0.25) / rad
green = np.array([0.12, 1.0, 0.22]) * np.clip(1.15 - dist * 0.45, 0.6, 1.0)[..., None]
hot = np.clip(1 - dist / 0.35, 0, 1)[..., None] * 0.85
col = green * (1 - hot) + np.array([0.85, 1.0, 0.85]) * hot
rgb = np.where(lamp[..., None], np.clip(col, 0, 1), rgb)
other = (~lamp) & (al > 0.5) & (s > 0.35) & (mx > 0.25) & (yy < cy - rad)
rgb = np.where(other[..., None], rgb * 0.45, rgb)
Image.fromarray((np.concatenate([rgb, al[..., None]], -1) * 255).astype(np.uint8), "RGBA").save(p)
print("зелёный свет исправлен")
