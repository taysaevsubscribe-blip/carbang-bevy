"""Перегенерированные текстуры v3b: hazard, asphalt_wrecked, mud, skid strip.

- убирает виньетку (тёмные углы) делением на сильно размытую яркость;
- делает плитку бесшовной: смещение на полкадра и подмешивание по краям;
- строит normal (OpenGL) и roughness по яркости;
- след шин: хромакей, вырезка полосы, бесшовность по горизонтали.

Запуск: python3 tools/textures/prepare_v3b.py hazard.png asphalt_wrecked.png mud.png skid.png
"""
import os, sys
import numpy as np
from PIL import Image, ImageFilter

sys.path.insert(0, os.path.dirname(__file__))
from prepare_v3 import key_green

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
A = os.path.join(ROOT, "assets")


def blur(a, r):
    im = Image.fromarray((np.clip(a, 0, 1) * 255).astype(np.uint8))
    return np.asarray(im.filter(ImageFilter.GaussianBlur(r))).astype(np.float64) / 255


def flatten(a, r=160):
    """Ровная освещённость: цвет делится на размытую яркость."""
    lum = a @ np.array([0.2126, 0.7152, 0.0722])
    low = blur(lum, r)
    k = low.mean() / np.maximum(low, 0.02)
    return np.clip(a * k[..., None], 0, 1)


def seamless(a, feather=0.18):
    h, w = a.shape[:2]
    rolled = np.roll(np.roll(a, h // 2, 0), w // 2, 1)
    y = np.minimum(np.arange(h), h - 1 - np.arange(h)) / (h * feather)
    x = np.minimum(np.arange(w), w - 1 - np.arange(w)) / (w * feather)
    wgt = np.clip(np.minimum.outer(y, x), 0, 1)
    if a.ndim == 3:
        wgt = wgt[..., None]
    return a * wgt + rolled * (1 - wgt)


def save_pbr(a, set_name, rough_base=0.8, metal=None):
    out = os.path.join(A, "textures", set_name)
    os.makedirs(out, exist_ok=True)
    Image.fromarray((a * 255).astype(np.uint8)).save(os.path.join(out, f"{set_name}_1K-JPG_Color.jpg"), quality=90)
    lum = a @ np.array([0.2126, 0.7152, 0.0722])
    h = blur(lum, 1.5)
    # высота считается по деталям, без крупных пятен
    h = h - blur(h, 24)
    gx = (np.roll(h, -1, 1) - np.roll(h, 1, 1)) * 6.0
    gy = (np.roll(h, -1, 0) - np.roll(h, 1, 0)) * 6.0
    n = np.stack([-gx, gy, np.ones_like(h)], -1)
    n /= np.linalg.norm(n, axis=-1, keepdims=True)
    Image.fromarray(((n * 0.5 + 0.5) * 255).astype(np.uint8)).save(os.path.join(out, f"{set_name}_1K-JPG_NormalGL.jpg"), quality=92)
    rough = np.clip(rough_base - h * 1.5, 0.2, 1.0)
    Image.fromarray((rough * 255).astype(np.uint8)).save(os.path.join(out, f"{set_name}_1K-JPG_Roughness.jpg"), quality=88)
    if metal is not None:
        Image.fromarray((np.clip(metal, 0, 1) * 255).astype(np.uint8)).save(os.path.join(out, f"{set_name}_1K-JPG_Metalness.jpg"), quality=88)


def main():
    hz, asph, mud, skid = sys.argv[1:5]
    load = lambda p: np.asarray(Image.open(p).convert("RGB").resize((1024, 1024), Image.LANCZOS)).astype(np.float64) / 255
    # полосы по стали: голый металл (серый, без насыщенности) металлический
    a = seamless(flatten(load(hz), 220))
    sat = a.max(-1) - a.min(-1)
    metal = np.where((sat < 0.08) & (a.mean(-1) > 0.35), 0.8, 0.1)
    save_pbr(a, "env_hazard", 0.6, metal)
    a = seamless(flatten(load(asph), 140))
    save_pbr(a, "env_asphalt_wrecked", 0.85)
    a = seamless(flatten(load(mud), 140))
    save_pbr(a, "env_mud", 0.55)
    # след шин: полоса по высоте содержимого, бесшовная по горизонтали
    rgb, al = key_green(Image.open(skid))
    rows = np.where(al.mean(1) > 0.05)[0]
    y0, y1 = max(rows.min() - 4, 0), min(rows.max() + 4, al.shape[0])
    rgb, al = rgb[y0:y1], al[y0:y1]
    w = al.shape[1]
    f = int(w * 0.12)
    rolled_rgb, rolled_al = np.roll(rgb, w // 2, 1), np.roll(al, w // 2, 1)
    x = np.clip(np.minimum(np.arange(w), w - 1 - np.arange(w)) / f, 0, 1)
    rgb = rgb * x[None, :, None] + rolled_rgb * (1 - x[None, :, None])
    al = al * x[None, :] + rolled_al * (1 - x[None, :])
    img = np.concatenate([np.clip(rgb, 0, 1), al[..., None]], -1)
    im = Image.fromarray((img * 255).astype(np.uint8), "RGBA").resize((1024, 256), Image.LANCZOS)
    im.save(os.path.join(A, "decals", "skid_strip.png"))
    print("готово")


if __name__ == "__main__":
    main()
