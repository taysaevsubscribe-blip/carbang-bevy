"""Текстуры кузовов из присланных материалов (CarBang_textures_v3) + процедурный слой.

assets/cars/tex/:
- paint_color.png   — износ краски (множитель к цвету скина): разводы, потёки грязи (Leaking012A),
                      сколы до грунта, царапины (car_damage_1 из пакета v3, хромакей);
- paint_mr.png      — шероховатость (G) и металличность (B): лак глянцевый, царапины и грязь матовые;
- paint_n.png       — нормали: «апельсиновая корка» лака, борозды царапин;
- colormap_mr.png   — шероховатость/металл по палитре Kenney: резина, пластик, хром;
- armor_*.png       — рифлёная сталь бамперов (env_diamond_plate из v3, исходник без стилизации).
Развёртка краски и брони кубическая (UV1 конвертера, 2 м на повтор).

Запуск: python3 tools/cars/car_textures.py [car_damage_1.png; по умолчанию копия в tools/cars/src]
"""
import os, sys
import numpy as np
from PIL import Image, ImageFilter

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
sys.path.insert(0, os.path.join(ROOT, "tools", "textures"))
from prepare_v3 import key_green

OUT = os.path.join(ROOT, "assets", "cars", "tex")
S = 1024


def value_noise(size, cells, seed):
    """Бесшовный шум значений: случайная сетка cells x cells, бикубическое увеличение с заворотом."""
    g = np.random.default_rng(seed).random((cells, cells))
    big = Image.fromarray((np.tile(g, (3, 3)) * 255).astype(np.uint8)).resize((size * 3, size * 3), Image.BICUBIC)
    return np.asarray(big).astype(np.float64)[size:2 * size, size:2 * size] / 255 * 2 - 1


def fbm(size, octaves, base, seed):
    out, amp, tot = np.zeros((size, size)), 1.0, 0.0
    for o in range(octaves):
        out += amp * value_noise(size, base * 2 ** o, seed + o)
        tot += amp
        amp *= 0.5
    return out / tot * 1.6


def blur(a, r):
    im = Image.fromarray((np.clip(a, 0, 1) * 255).astype(np.uint8))
    return np.asarray(im.filter(ImageFilter.GaussianBlur(r))).astype(np.float64) / 255


def tile_blur(a, r):
    """Размытие с заворотом краёв, чтобы плитка оставалась бесшовной."""
    h, w = a.shape
    big = np.tile(a, (3, 3))
    return blur(big, r)[h:2 * h, w:2 * w]


def height_to_normal(h, k):
    gx = (np.roll(h, -1, 1) - np.roll(h, 1, 1)) * k
    gy = (np.roll(h, -1, 0) - np.roll(h, 1, 0)) * k
    n = np.stack([-gx, gy, np.ones_like(h)], -1)
    n /= np.linalg.norm(n, axis=-1, keepdims=True)
    return Image.fromarray(((n * 0.5 + 0.5) * 255).round().astype(np.uint8))


def save(a, name, mode="L"):
    Image.fromarray((np.clip(a, 0, 1) * 255).round().astype(np.uint8), mode).save(os.path.join(OUT, name))


def scratches(src):
    """Царапины из присланной наклейки лёгких повреждений: маска 0..1, бесшовно в 4 поворотах."""
    rgb, al = key_green(Image.open(src))
    al = np.asarray(Image.fromarray((al * 255).astype(np.uint8)).resize((S, S), Image.LANCZOS)).astype(np.float64) / 255
    m = np.zeros((S, S))
    for k, (dy, dx) in enumerate([(0, 0), (S // 2, S // 3), (S // 5, S // 2), (S * 2 // 3, S // 6)]):
        m = np.maximum(m, np.roll(np.roll(np.rot90(al, k), dy, 0), dx, 1))
    return np.clip(m * 1.2, 0, 1)


def main():
    os.makedirs(OUT, exist_ok=True)
    dmg1 = sys.argv[1] if len(sys.argv) > 1 else os.path.join(os.path.dirname(__file__), "src", "car_damage_1.png")
    # потёки: карта Leaking012A (ambientCG, CC0), вертикальные полосы
    leak = np.asarray(Image.open(os.path.join(ROOT, "assets/textures/Leaking012A/Leaking012A_1K-JPG_Roughness.jpg")).convert("L").resize((S, S), Image.LANCZOS)).astype(np.float64) / 255
    streak = np.clip((0.75 - leak) / 0.5, 0, 1)
    mott = fbm(S, 4, 2, 11)                  # крупные разводы
    grime = np.clip(0.5 + 0.5 * fbm(S, 5, 3, 23), 0, 1) ** 2.2
    sc = scratches(dmg1)
    # сколы: мелкие пятна с порогом
    chip_n = fbm(S, 3, 32, 37)
    chips = np.clip((chip_n - 0.62) / 0.05, 0, 1)
    chips = chips * np.clip((grime - 0.5) * 4, 0, 1)
    # цвет: множитель к краске; материал умножает цвет краски на 1.12, чистый лак ~0.9
    col = np.full((S, S, 3), 0.9)
    col *= (1 + 0.04 * mott)[..., None]
    col *= (1 - 0.22 * streak * grime)[..., None]
    col = col * (1 - grime * 0.12)[..., None] + np.array([0.06, 0.05, 0.03]) * grime[..., None]
    primer = np.array([0.55, 0.55, 0.52])
    col = col * (1 - chips[..., None]) + primer * chips[..., None] * 0.9
    col = col * (1 - sc[..., None] * 0.5) + np.array([0.86, 0.86, 0.86]) * sc[..., None] * 0.5
    save(col, "paint_color.png", "RGB")
    # шероховатость и металл (glTF: G — шероховатость, B — металл)
    rough = 0.13 + 0.04 * mott + 0.35 * grime * (0.4 + 0.6 * streak) + 0.45 * sc + 0.5 * chips
    metal = 0.55 - 0.35 * grime + 0.4 * sc - 0.5 * chips
    mr = np.stack([np.ones((S, S)), np.clip(rough, 0.08, 1), np.clip(metal, 0, 1)], -1)
    save(mr, "paint_mr.png", "RGB")
    # нормали: лак с «апельсиновой коркой», борозды царапин, края сколов
    peel = fbm(S, 2, 96, 51) * 0.5
    h = peel * 0.02 - tile_blur(sc, 0.8) * 0.5 - tile_blur(chips, 1.2) * 0.6
    height_to_normal(h, 6.0).save(os.path.join(OUT, "paint_n.png"))
    # палитра Kenney -> шероховатость/металл
    cm = np.asarray(Image.open(os.path.join(ROOT, "assets/cars/colormap.png")).convert("RGB")).astype(np.float64) / 255
    mx, mn = cm.max(-1), cm.min(-1)
    sat = (mx - mn) / np.maximum(mx, 1e-3)
    lum = cm @ np.array([0.2126, 0.7152, 0.0722])
    gray = sat < 0.2
    r = np.where(gray, np.where(lum < 0.3, 0.82, np.where(lum < 0.62, 0.5, 0.22)), 0.38)
    m = np.where(gray, np.where(lum < 0.3, 0.0, np.where(lum < 0.62, 0.35, 0.9)), 0.2)
    save(np.stack([np.ones_like(r), r, m], -1), "colormap_mr.png", "RGB")
    # броня бамперов: рифлёная сталь, исходник v3 без стилизации, 512 px
    src = os.path.join(ROOT, "art_src/photo/textures/env_diamond_plate/env_diamond_plate_1K-JPG_")
    c = Image.open(src + "Color.jpg").convert("RGB").resize((512, 512), Image.LANCZOS)
    c.save(os.path.join(OUT, "armor_color.png"))
    Image.open(src + "NormalGL.jpg").convert("RGB").resize((512, 512), Image.LANCZOS).save(os.path.join(OUT, "armor_n.png"))
    ro = np.asarray(Image.open(src + "Roughness.jpg").convert("L").resize((512, 512), Image.LANCZOS)).astype(np.float64) / 255
    save(np.stack([np.ones_like(ro), np.clip(ro * 0.8, 0.2, 1), np.full_like(ro, 0.85)], -1), "armor_mr.png", "RGB")
    print("готово:", sorted(os.listdir(OUT)))


if __name__ == "__main__":
    main()
