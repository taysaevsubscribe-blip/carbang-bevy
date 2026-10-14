"""Пакет «klod v1» (Ideogram) -> assets.

- плашки всплывашек, светофор старта, иконки бонусов, граффити: зелёный фон -> альфа;
- спрайты эффектов на чёрном фоне: альфа по яркости, листы 2x2 режутся на кадры;
- обломки и болты: зелёный фон, листы 2x2;
- фасады и покрытия: цвет + карты нормалей и шероховатости по яркости, ночная маска окон;
- плакаты: как есть (бумага с краями).

Запуск: python3 tools/textures/prepare_fx.py <папка с png>
"""
import os, sys
import numpy as np
from PIL import Image, ImageFilter

sys.path.insert(0, os.path.dirname(__file__))
from prepare_v3 import key_green, crop_content, to_rgba

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
A = os.path.join(ROOT, "assets")


def fit_square(im, size):
    """Вписать RGBA в квадрат size с прозрачными полями."""
    im = im.copy()
    im.thumbnail((size, size), Image.LANCZOS)
    out = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    out.paste(im, ((size - im.width) // 2, (size - im.height) // 2), im)
    return out


def keyed(src, size, margin=0.03):
    rgb, al = key_green(Image.open(src))
    rgb, al = crop_content(rgb, al, margin)
    return fit_square(to_rgba(rgb, al, 0), size)


def glow_alpha(rgb, gamma=0.8, gain=1.0):
    """Чёрный фон -> альфа по максимуму каналов, цвет без умножения на альфу."""
    a = np.clip(rgb.max(-1) * gain, 0, 1) ** gamma
    col = np.clip(rgb / np.maximum(a[..., None], 1e-3), 0, 1)
    a[a < 0.02] = 0
    return col, a


def black_sprite(arr, size, smoke=False):
    rgb = arr.astype(np.float64) / 255
    if smoke:
        lum = rgb @ np.array([0.3, 0.5, 0.2])
        a = np.clip(lum * 1.25, 0, 1)
        col = np.clip(rgb / np.maximum(a[..., None], 1e-3), 0, 1)
    else:
        col, a = glow_alpha(rgb)
    # мягкий край, чтобы рамка кадра не была видна
    h, w = a.shape
    yy, xx = np.mgrid[0:h, 0:w]
    edge = np.minimum.reduce([xx, yy, w - 1 - xx, h - 1 - yy]) / (0.06 * w)
    a = a * np.clip(edge, 0, 1)
    im = to_rgba(col, a, 0)
    return im.resize((size, size), Image.LANCZOS)


def split4(path):
    im = np.asarray(Image.open(path).convert("RGB"))
    h, w, _ = im.shape
    m = int(w * 0.012)  # без разделительных линий между кадрами
    return [im[r * h // 2 + m:(r + 1) * h // 2 - m, c * w // 2 + m:(c + 1) * w // 2 - m] for r in range(2) for c in range(2)]


def pbr_from_color(src, set_name, tile_square=True, night_mask=False):
    im = Image.open(src).convert("RGB")
    out = os.path.join(A, "textures", set_name)
    os.makedirs(out, exist_ok=True)
    im.save(os.path.join(out, f"{set_name}_1K-JPG_Color.jpg"), quality=90)
    a = np.asarray(im).astype(np.float64) / 255
    lum = a @ np.array([0.2126, 0.7152, 0.0722])
    h = np.asarray(Image.fromarray((lum * 255).astype(np.uint8)).filter(ImageFilter.GaussianBlur(1.5))).astype(np.float64) / 255
    gx = (np.roll(h, -1, 1) - np.roll(h, 1, 1)) * 3.0
    gy = (np.roll(h, -1, 0) - np.roll(h, 1, 0)) * 3.0
    n = np.stack([-gx, gy, np.ones_like(h)], -1)  # OpenGL: +Y вверх
    n /= np.linalg.norm(n, axis=-1, keepdims=True)
    Image.fromarray(((n * 0.5 + 0.5) * 255).astype(np.uint8)).save(os.path.join(out, f"{set_name}_1K-JPG_NormalGL.jpg"), quality=92)
    rough = np.clip(0.95 - 0.35 * (h - h.mean()) / (h.std() + 1e-6) * 0.25, 0.45, 1.0)
    Image.fromarray((rough * 255).astype(np.uint8)).save(os.path.join(out, f"{set_name}_1K-JPG_Roughness.jpg"), quality=88)
    if night_mask:
        # светящиеся тёплые окна: яркие и желтоватые пиксели
        r, g, b = a[..., 0], a[..., 1], a[..., 2]
        lit = (lum > 0.5) & (r > b + 0.08) & (g > b)
        m = np.zeros_like(a)
        m[lit] = [1.0, 0.8, 0.5]
        m = np.asarray(Image.fromarray((m * 255).astype(np.uint8)).filter(ImageFilter.GaussianBlur(0.8)))
        Image.fromarray(m).save(os.path.join(out, f"{set_name}_emissive.png"))
        return lit.mean()


def main():
    src = sys.argv[1]
    p = lambda f: os.path.join(src, f)
    names = sorted(os.listdir(src))
    # плашки и старт
    d = os.path.join(A, "ui", "pop"); os.makedirs(d, exist_ok=True)
    for f in names:
        if f.startswith(("pop_", "start_")):
            keyed(p(f), 512).save(os.path.join(d, f))
    # иконки бонусов
    d = os.path.join(A, "ui", "powerups"); os.makedirs(d, exist_ok=True)
    for f in names:
        if f.startswith("ico_"):
            keyed(p(f), 256, 0.06).save(os.path.join(d, f))
    # граффити -> декали
    d = os.path.join(A, "decals"); os.makedirs(d, exist_ok=True)
    for f in names:
        if f.startswith("graffiti_"):
            keyed(p(f), 512).save(os.path.join(d, f))
        if f.startswith("poster_"):
            Image.open(p(f)).convert("RGB").resize((512, 512), Image.LANCZOS).save(os.path.join(d, f.replace(".png", ".jpg")), quality=88)
    # эффекты
    d = os.path.join(A, "fx"); os.makedirs(d, exist_ok=True)
    for base, smoke in [("explosion", False), ("fire", False), ("smoke", True), ("dust", True), ("muzzle", False)]:
        for i, cell in enumerate(split4(p(f"fx_{base}_2x2.png"))):
            black_sprite(cell, 256, smoke).save(os.path.join(d, f"{base}_{i}.png"))
    for base, size in [("sparks", 512), ("flash", 256), ("shockwave", 512), ("repair_glow", 256)]:
        black_sprite(np.asarray(Image.open(p(f"fx_{base}.png")).convert("RGB")), size).save(os.path.join(d, f"{base}.png"))
    for base in ["debris", "bolts"]:
        for i, cell in enumerate(split4(p(f"fx_{base}_2x2.png"))):
            rgb, al = key_green(Image.fromarray(cell))
            rgb, al = crop_content(rgb, al, 0.04)
            fit_square(to_rgba(rgb, al, 0), 256).save(os.path.join(d, f"{base}_{i}.png"))
    # фасады и покрытия
    lit = pbr_from_color(p("env_facade_brick_windows.png"), "env_facade_brick", night_mask=True)
    lit2 = pbr_from_color(p("env_facade_concrete_windows.png"), "env_facade_concrete", night_mask=True)
    for f, n in [("env_cobble.png", "env_cobble"), ("env_grating.png", "env_grating"), ("env_warehouse_floor.png", "env_warehouse_floor")]:
        pbr_from_color(p(f), n)
    print("готово; доля светящихся окон:", round(lit, 3), round(lit2, 3))


if __name__ == "__main__":
    main()
