"""Стилизация текстур под плоские модели Kenney.

Фото-текстуры (асфальт, бетон, грунт, фасады, декали, повреждения, спрайты огня и дыма)
упрощаются: сглаживание с сохранением краёв, постеризация в 8-14 цветов, чуть больше
насыщенности. Рельеф нормалей ослабляется, альфа декалей и спрайтов становится чётче.
Исходники сохраняются в art_src/photo (в сборку не попадают), повторный запуск берёт их.

Запуск: python3 tools/textures/stylize.py
"""
import glob, os, shutil
import numpy as np
from PIL import Image, ImageEnhance, ImageFilter

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
A = os.path.join(ROOT, "assets")
BACKUP = os.path.join(ROOT, "art_src", "photo")


def source(path):
    """Путь к фото-исходнику: при первом запуске копирует файл в art_src/photo."""
    rel = os.path.relpath(path, A)
    b = os.path.join(BACKUP, rel)
    if not os.path.exists(b):
        os.makedirs(os.path.dirname(b), exist_ok=True)
        shutil.copy2(path, b)
    return b


def stylize_rgb(im, colors=8, sat=1.15, contrast=1.06, smooth=5, detail=0.4, outline=0.14):
    """Мелкое зерно приглушается (detail), крупные формы остаются; постеризация и тонкий контур."""
    im = im.convert("RGB")
    a = np.asarray(im).astype(np.float64)
    low = np.asarray(im.filter(ImageFilter.GaussianBlur(5))).astype(np.float64)
    a = np.clip(low + (a - low) * detail, 0, 255).astype(np.uint8)
    sm = Image.fromarray(a).filter(ImageFilter.MedianFilter(smooth)).filter(ImageFilter.MedianFilter(smooth))
    qi = sm.quantize(colors=colors, method=Image.Quantize.MEDIANCUT, dither=Image.Dither.NONE)
    q = np.asarray(qi.convert("RGB")).astype(np.float64)
    if outline > 0:
        idx = np.asarray(qi).astype(np.int32)
        edge = (idx != np.roll(idx, 1, 0)) | (idx != np.roll(idx, 1, 1))
        lum = q.mean(-1)
        # контур только на сильных перепадах яркости, чтобы не рябило
        strong = np.abs(lum - np.roll(lum, 1, 0)) + np.abs(lum - np.roll(lum, 1, 1)) > 40
        q[edge & strong] *= 1 - outline
    out = Image.fromarray(np.clip(q, 0, 255).astype(np.uint8))
    out = ImageEnhance.Color(out).enhance(sat)
    out = ImageEnhance.Contrast(out).enhance(contrast)
    return out


def soften_normal(im, k=0.45):
    a = np.asarray(im.convert("RGB").filter(ImageFilter.GaussianBlur(1.6))).astype(np.float64) / 255 * 2 - 1
    flat = np.array([0.0, 0.0, 1.0])
    n = a * k + flat * (1 - k)
    n /= np.linalg.norm(n, axis=-1, keepdims=True)
    return Image.fromarray(((n * 0.5 + 0.5) * 255).astype(np.uint8))


def crisp_alpha(a, lo=0.3, hi=0.55, levels=0):
    a = np.clip((a - lo) / (hi - lo), 0, 1)
    if levels:
        a = np.round(a * levels) / levels
    return a


def stylize_rgba(im, colors=10, levels=0, lo=0.3, hi=0.55, sat=1.15):
    im = im.convert("RGBA")
    rgb = stylize_rgb(im.convert("RGB"), colors=colors, sat=sat, smooth=3, detail=0.6, outline=0.0)
    a = np.asarray(im)[..., 3].astype(np.float64) / 255
    a = crisp_alpha(a, lo, hi, levels)
    out = np.concatenate([np.asarray(rgb), (a[..., None] * 255).astype(np.uint8)], -1)
    return Image.fromarray(out.astype(np.uint8), "RGBA")


def main():
    n = 0
    # PBR-наборы окружения
    for d in sorted(glob.glob(os.path.join(A, "textures", "*"))):
        name = os.path.basename(d)
        if not (name.startswith("env_") or name in ("ManholeCover011", "RoadLines019A")):
            continue
        for f in glob.glob(os.path.join(d, "*_1K-JPG_Color.jpg")):
            facade = "facade" in name
            stylize_rgb(Image.open(source(f)), colors=12 if facade else 8, sat=1.12 if facade else 1.2, detail=0.6 if facade else 0.35).save(f, quality=90)
            n += 1
        for f in glob.glob(os.path.join(d, "*_1K-JPG_NormalGL.jpg")):
            soften_normal(Image.open(source(f))).save(f, quality=92)
        for f in glob.glob(os.path.join(d, "*_1K-JPG_Roughness.jpg")):
            Image.open(source(f)).convert("L").filter(ImageFilter.MedianFilter(5)).save(f, quality=88)
    # декали на земле и наклейки повреждений
    for f in glob.glob(os.path.join(A, "decals", "decal_*.png")) + glob.glob(os.path.join(A, "damage", "mark_*.png")) + glob.glob(os.path.join(A, "damage", "damage_stage*.png")):
        stylize_rgba(Image.open(source(f)), colors=8).save(f)
        n += 1
    # спрайты: огонь и взрывы ступенями яркости, дым и пыль ступенями прозрачности
    for f in glob.glob(os.path.join(A, "fx", "*.png")):
        b = os.path.basename(f)
        if b.startswith(("fire", "explosion", "muzzle")):
            stylize_rgba(Image.open(source(f)), colors=6, levels=4, lo=0.05, hi=0.6, sat=1.3).save(f)
        elif b.startswith(("smoke", "dust")):
            stylize_rgba(Image.open(source(f)), colors=5, levels=3, lo=0.22, hi=0.8, sat=1.0).save(f)
        elif b.startswith(("debris", "bolts")):
            stylize_rgba(Image.open(source(f)), colors=8).save(f)
        else:
            continue
        n += 1
    print("стилизовано файлов:", n, "| исходники:", os.path.relpath(BACKUP, ROOT))


if __name__ == "__main__":
    main()
