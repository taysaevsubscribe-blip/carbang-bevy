"""Маски ночных окон для наборов City Kit: голубые ячейки палитры (стекло) -> тёплый свет.
Запуск: python3 tools/kits/window_masks.py"""
import colorsys, os
import numpy as np
from PIL import Image
ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
for kit in ["city_commercial", "city_industrial", "city_suburban"]:
    d = os.path.join(ROOT, "assets", "kenney", kit, "Textures")
    a = np.asarray(Image.open(os.path.join(d, "colormap.png")).convert("RGB")).astype(np.float64) / 255
    h, w, _ = a.shape
    out = np.zeros_like(a)
    hsv = np.array([colorsys.rgb_to_hsv(*px) for px in a.reshape(-1, 3)]).reshape(h, w, 3)
    hue, sat, val = hsv[..., 0] * 360, hsv[..., 1], hsv[..., 2]
    glass = (hue > 200) & (hue < 235) & (sat > 0.18) & (sat < 0.75) & (val > 0.6)
    out[glass] = [1.0, 0.78, 0.45]
    Image.fromarray((out * 255).astype(np.uint8)).save(os.path.join(d, "emissive_windows.png"))
    print(kit, "доля стекла", round(glass.mean(), 3))
