"""Текстуры v3 (Ideogram + карты пользователя) -> assets.

1. Наборы окружения env_* -> assets/textures/<set>/<set>_1K-JPG_<Kind>.jpg (формат загрузчика ambientCG).
2. Повреждения кузова car_damage_1..4 (зелёный фон) -> бесшовные слои assets/damage/damage_stage1..4.png.
3. Декали scorch/crater/oil/skid/glass (зелёный фон) -> assets/decals/*.png с альфой.

Запуск: python3 tools/textures/prepare_v3.py <распакованный _t>
"""
import os, sys, shutil
import numpy as np
from PIL import Image

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
A = os.path.join(ROOT, "assets")
KINDS = {"color": "Color", "normal": "NormalGL", "roughness": "Roughness", "ao": "AmbientOcclusion", "metalness": "Metalness"}
# Табличка DANGER вместо полос и асфальт с разметкой в игру не идут
SKIP = {"env_hazard", "env_asphalt_wrecked"}


def env_sets(src):
    done = []
    for d in sorted(os.listdir(os.path.join(src, "env"))):
        if d in SKIP:
            continue
        out = os.path.join(A, "textures", d)
        os.makedirs(out, exist_ok=True)
        for k, K in KINDS.items():
            p = os.path.join(src, "env", d, f"{d}_{k}.jpg")
            if not os.path.exists(p):
                continue
            im = Image.open(p)
            if k == "metalness" and np.asarray(im.convert("L")).max() < 8:
                continue  # пустая карта металла
            im = im.convert("RGB" if k in ("color", "normal") else "L")
            if im.height * 2 == im.width:
                # 1024x512: бесшовна по вертикали, повторяем до квадрата (иначе кирпич растянется)
                sq = Image.new(im.mode, (im.width, im.width))
                sq.paste(im, (0, 0))
                sq.paste(im, (0, im.height))
                im = sq
            im.save(os.path.join(out, f"{d}_1K-JPG_{K}.jpg"), quality=88 if k != "normal" else 92)
        done.append(d)
    return done


def key_green(im):
    """Хромакей #00FF00: альфа по перевесу зелёного, подавление зелёного ореола."""
    a = np.asarray(im.convert("RGB")).astype(np.float64) / 255.0
    r, g, b = a[..., 0], a[..., 1], a[..., 2]
    spill = g - np.maximum(r, b)
    alpha = 1.0 - np.clip((spill - 0.06) / 0.32, 0.0, 1.0)
    g2 = np.minimum(g, np.maximum(r, b) * 1.04)
    rgb = np.stack([r, g2, b], -1)
    return rgb, alpha


def crop_content(rgb, alpha, margin=0.06):
    ys, xs = np.where(alpha > 0.08)
    if len(xs) == 0:
        return rgb, alpha
    x0, x1, y0, y1 = xs.min(), xs.max(), ys.min(), ys.max()
    h, w = alpha.shape
    s = int(max(x1 - x0, y1 - y0) * (1 + 2 * margin))
    cx, cy = (x0 + x1) // 2, (y0 + y1) // 2
    X0, Y0 = max(0, cx - s // 2), max(0, cy - s // 2)
    X1, Y1 = min(w, X0 + s), min(h, Y0 + s)
    return rgb[Y0:Y1, X0:X1], alpha[Y0:Y1, X0:X1]


def to_rgba(rgb, alpha, size):
    img = np.concatenate([rgb, alpha[..., None]], -1)
    im = Image.fromarray((np.clip(img, 0, 1) * 255).round().astype(np.uint8), "RGBA")
    return im.resize((size, size), Image.LANCZOS) if size else im


def decals(src):
    out = os.path.join(A, "decals")
    os.makedirs(out, exist_ok=True)
    names = []
    for f in sorted(os.listdir(os.path.join(src, "decals"))):
        rgb, al = key_green(Image.open(os.path.join(src, "decals", f)))
        rgb, al = crop_content(rgb, al)
        # мягкий круглый край, чтобы не было квадратной рамки
        h, w = al.shape
        yy, xx = np.mgrid[0:h, 0:w]
        rr = np.hypot((xx - w / 2) / (w / 2), (yy - h / 2) / (h / 2))
        al = al * np.clip((1.0 - rr) / 0.12, 0, 1)
        to_rgba(rgb, al, 512).save(os.path.join(out, f))
        names.append(f)
    return names


def paste_wrap(dst_rgb, dst_a, rgb, al, x, y, opacity=1.0):
    """Наложение с переносом через края (бесшовная плитка)."""
    H, W = dst_a.shape
    h, w = al.shape
    for oy in (0, -H, H):
        for ox in (0, -W, W):
            X0, Y0 = x + ox, y + oy
            xs0, ys0 = max(0, X0), max(0, Y0)
            xs1, ys1 = min(W, X0 + w), min(H, Y0 + h)
            if xs0 >= xs1 or ys0 >= ys1:
                continue
            sa = al[ys0 - Y0:ys1 - Y0, xs0 - X0:xs1 - X0] * opacity
            sr = rgb[ys0 - Y0:ys1 - Y0, xs0 - X0:xs1 - X0]
            da = dst_a[ys0:ys1, xs0:xs1]
            na = sa + da * (1 - sa)
            dst_rgb[ys0:ys1, xs0:xs1] = (sr * sa[..., None] + dst_rgb[ys0:ys1, xs0:xs1] * (da * (1 - sa))[..., None]) / np.maximum(na, 1e-6)[..., None]
            dst_a[ys0:ys1, xs0:xs1] = na


def damage_stages(src):
    S = 1024
    rng = np.random.default_rng(7)
    parts = []
    for i in range(1, 5):
        rgb, al = key_green(Image.open(os.path.join(src, "damage", f"car_damage_{i}.png")))
        if i == 3:
            # у третьей картинки серый прямоугольник вокруг дыр: мягкая овальная маска
            h, w = al.shape
            yy, xx = np.mgrid[0:h, 0:w]
            ys, xs = np.where(al > 0.5)
            cx, cy = xs.mean(), ys.mean()
            rx, ry = (xs.max() - xs.min()) / 2, (ys.max() - ys.min()) / 2
            rr = np.hypot((xx - cx) / rx, (yy - cy) / ry)
            al = al * np.clip((1.0 - rr) / 0.35, 0, 1)
        parts.append(crop_content(rgb, al, 0.02))
        # отдельная наклейка повреждения для моделей Kenney (дверь, капот)
        r2, a2 = parts[-1]
        h, w = a2.shape
        yy, xx = np.mgrid[0:h, 0:w]
        rr = np.hypot((xx - w / 2) / (w / 2), (yy - h / 2) / (h / 2))
        a2 = a2 * np.clip((1.0 - rr) / 0.25, 0, 1)
        im = to_rgba(r2, a2, 0)
        im.thumbnail((512, 512), Image.LANCZOS)
        im.save(os.path.join(A, "damage", f"mark_{i}.png"))

    def scaled(k, size, rot):
        rgb, al = parts[k]
        im = to_rgba(rgb, al, 0).rotate(rot, expand=True, resample=Image.BICUBIC)
        im = im.resize((size, max(1, int(size * im.height / im.width))), Image.LANCZOS)
        a = np.asarray(im).astype(np.float64) / 255.0
        return a[..., :3], a[..., 3]

    canvas_rgb = np.zeros((S, S, 3))
    canvas_a = np.zeros((S, S))
    plan = {
        1: [(0, 520, 3), (0, 420, 2)],
        2: [(1, 560, 2)],
        3: [(2, 520, 2)],
    }
    out = os.path.join(A, "damage")
    for stage in range(1, 5):
        if stage == 4:
            # стадия 4: целиком картинка «разбита» плиткой со сдвигом поверх стадии 3
            rgb, al = scaled(3, 900, 0)
            for (x, y) in [(60, 60), (560, 520)]:
                paste_wrap(canvas_rgb, canvas_a, rgb, al, x, y, 0.95)
        else:
            for k, size, n in plan[stage]:
                for _ in range(n):
                    rgb, al = scaled(k, int(size * rng.uniform(0.75, 1.1)), float(rng.uniform(0, 360)))
                    paste_wrap(canvas_rgb, canvas_a, rgb, al, int(rng.uniform(0, S)), int(rng.uniform(0, S)), 0.92)
        to_rgba(canvas_rgb, canvas_a, 0).save(os.path.join(out, f"damage_stage{stage}.png"), optimize=True)


def main():
    src = sys.argv[1]
    sets = env_sets(src)
    print("наборы:", ", ".join(sets))
    print("декали:", ", ".join(decals(src)))
    damage_stages(src)
    print("стадии износа: 4")


if __name__ == "__main__":
    main()
