"""Kenney Car Kit (CC0) -> игровые меши CarBang (.cbm) и таблица src/carfit.rs.

Что делает с каждой моделью:
- разворачивает (у Kenney перед +Z, в игре -Z), масштабирует под размеры машины;
- «Мэр» удлиняет до лимузина, раздвигая кузов посередине;
- делит кузов по ячейкам палитры: краска (перекрашивается скином), отделка, стекло;
- вырезает бамперы в отрывные детали, двери/ковш тоже отрывные;
- колёса отдельными частями с центром в оси (крутятся и поворачиваются в игре);
- добавляет UV1 (кубическая проекция, 1 повтор на 2 м) для слоёв износа и трещин.

Запуск: python3 tools/cars/convert.py [assets/kenney/car]
(соответствие машинам игры задано пользователем в CarBang_kenney_12cars.zip, car12 «запасная» не назначена)
"""
import json, os, struct, sys
import numpy as np
from PIL import Image

sys.path.insert(0, os.path.dirname(__file__))
from glb import mesh_nodes
from bake import bake_parts

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
OUT = os.path.join(ROOT, "assets", "cars")

# id игры, файл, масштаб по длине/высоте, масштаб по ширине, удлинение (ед. модели), ячейка краски (строка, пара столбцов)
CARS = [
    ("ferret", "hatchback-sports", 1.37, 1.37, 0.0, (1, 3)),
    ("icecream", "delivery", 1.46, 1.40, 0.0, (1, 3)),
    ("buggy", "kart-oobi", 1.90, 1.88, 0.0, (3, 5)),
    ("golf", "sedan", 1.45, 1.30, 0.0, (1, 6)),
    ("hearse", "ambulance", 1.50, 1.37, 0.0, (2, 6)),
    ("bison", "truck", 1.62, 1.35, 0.0, (1, 3)),
    ("wasp", "race", 1.70, 1.62, 0.0, (1, 6)),
    ("limo", "suv-luxury", 1.40, 1.37, 1.72, (1, 4)),
    ("monster", "garbage-truck", 1.45, 1.45, 0.0, (1, 3)),
    ("bus", "suv", 1.45, 1.36, 0.0, (1, 3)),
    ("roller", "tractor-shovel", 1.60, 1.57, 0.0, (1, 3)),
    # Миксер: бортовой грузовик, удлинённая платформа, барабан добавляет игра
    ("mixer", "truck-flat", 1.60, 1.66, 2.0, (2, 6)),
]
GLASS = (3, 0)  # светло-голубые окна
HEAD = (3, 1)   # жёлтые фары
BRAKE = (3, 2)  # красные стопы
TRIM = (2, 3)   # тёмно-серые бамперы, пороги, арки
DETACH_NODES = {"door": 1, "door-left": 1, "door-right": 1, "shovel": 0}


def cell_of(uv):
    """(строка, пара столбцов 0..7) ячейки палитры 512x512: 8 пар по 64 px, 4 строки по 128 px."""
    return int(np.clip(uv[1] * 4, 0, 3)), int(np.clip(uv[0] * 8, 0, 7))


def paint_colormap(src, dst):
    """Серая карта для окрашиваемых ячеек: яркость пикселя относительно средней по ячейке."""
    a = np.asarray(Image.open(src).convert("RGB")).astype(np.float64) / 255.0
    lum = a @ np.array([0.2126, 0.7152, 0.0722])
    out = np.zeros_like(lum)
    for r in range(4):
        for c in range(8):
            blk = lum[r * 128:(r + 1) * 128, c * 64:(c + 1) * 64]
            m = max(blk.mean(), 1e-3)
            out[r * 128:(r + 1) * 128, c * 64:(c + 1) * 64] = np.clip(blk / m, 0.55, 1.3) / 1.3
    Image.fromarray((out * 255).round().astype(np.uint8), "L").convert("RGB").save(dst)


def face_normals(P):
    n = np.cross(P[:, 1] - P[:, 0], P[:, 2] - P[:, 0])
    ln = np.linalg.norm(n, axis=1, keepdims=True)
    return n / np.maximum(ln, 1e-12)


def box_uv(P, N):
    """UV1: проекция на плоскость, перпендикулярную главной оси нормали, 0.5 повтора на метр."""
    ax = np.abs(N).argmax(axis=1)
    uv = np.zeros(P.shape[:2] + (2,))
    for k in range(3):
        m = ax == k
        a, b = [(2, 1), (0, 2), (0, 1)][k]
        uv[m, :, 0] = P[m][:, :, a] * 0.5
        uv[m, :, 1] = -P[m][:, :, b] * 0.5
    return uv


def convert(gid, fname, s, sx, stretch, paint_cell, src_dir):
    scale = np.array([sx, s, s])
    tris = []  # (позиции 3x3, нормали 3x3, uv 3x2, класс)
    wheels = []
    for name, M, prims, chain in mesh_nodes(os.path.join(src_dir, fname + ".glb")):
        is_wheel = name.startswith("wheel-") and ("left" in name or "right" in name)
        for p in prims:
            P = (M[:3, :3] @ p["pos"].T).T + M[:3, 3]
            N = (M[:3, :3] @ p["nrm"].T).T
            if stretch:
                if is_wheel:
                    P[:, 2] += np.sign(M[2, 3]) * stretch / 2
                else:
                    P[:, 2] += np.where(P[:, 2] > 0, stretch / 2, -stretch / 2)
            # разворот на 180° вокруг Y и масштаб
            P = P * np.array([-1, 1, -1]) * scale
            N = N * np.array([-1, 1, -1]) / scale
            N /= np.maximum(np.linalg.norm(N, axis=1, keepdims=True), 1e-12)
            I = p["idx"].reshape(-1, 3)
            if is_wheel:
                wheels.append({"P": P[I], "N": N[I], "UV": p["uv"][I]})
                continue
            kind = "detach:%d" % DETACH_NODES[name] if name in DETACH_NODES else None
            for t in I:
                c = cell_of(p["uv"][t].mean(0))
                if kind:
                    k = kind
                elif c == paint_cell:
                    k = "paint"
                elif c == GLASS:
                    k = "glass"
                elif c == HEAD and P[t].mean(0)[2] < 0:
                    k = "head"
                elif c in (HEAD, BRAKE) and P[t].mean(0)[2] > 0:
                    k = "brake"
                else:
                    k = "trim"
                tris.append((P[t], N[t], p["uv"][t], k))
    # колёса: центр, радиус, порядок как у физики (0 перед-лево, 1 перед-право, 2 зад-лево, 3 зад-право)
    for w in wheels:
        allp = w["P"].reshape(-1, 3)
        mn, mx = allp.min(0), allp.max(0)
        w["c"] = (mn + mx) / 2
        w["r"] = (mx[1] - mn[1]) / 2
    def slot(w):
        return (0 if w["c"][2] < 0 else 2) + (0 if w["c"][0] < 0 else 1)
    wheels.sort(key=slot)
    assert [slot(w) for w in wheels] == [0, 1, 2, 3], (gid, [slot(w) for w in wheels])
    # начало координат: центр габарита кузова по X/Z, высота оси колёс = 0
    body_p = np.array([t[0] for t in tris]).reshape(-1, 3)
    mn, mx = body_p.min(0), body_p.max(0)
    origin = np.array([0.0, np.mean([w["c"][1] for w in wheels]), (mn[2] + mx[2]) / 2])
    tris = [(t[0] - origin, t[1], t[2], t[3]) for t in tris]
    for w in wheels:
        w["c"] = w["c"] - origin
    mn, mx = mn - origin, mx - origin
    L = mx[2] - mn[2]
    # бамперы: отделка в крайних 11% длины ниже оси колёс + 25% радиуса
    rw = np.mean([w["r"] for w in wheels])
    out = []
    for P, N, UV, k in tris:
        if k == "trim":
            c = P.mean(0)
            if c[1] < rw * 0.35:
                if c[2] < mn[2] + L * 0.11:
                    k = "detach:0"
                elif c[2] > mx[2] - L * 0.11:
                    k = "detach:1"
        out.append((P, N, UV, k))
    tris = out
    # крыша: верх кузова в средней трети
    mid = [t[0] for t in tris if abs(t[0].mean(0)[2]) < L * 0.17 and t[3] != "glass"]
    midp = np.array(mid).reshape(-1, 3) if mid else body_p
    roof_y = midp[:, 1].max()
    # сборка частей
    parts = []
    def add_part(name, P, N, UV, center=np.zeros(3), zone=-1):
        P = P - center
        FN = face_normals(P)
        N = np.where(np.isfinite(N), N, FN[:, None, :])
        uv1 = box_uv(P + center, FN)
        parts.append({"name": name, "zone": zone, "center": center, "P": P, "N": N, "UV": UV, "UV1": uv1})
    for k in ["paint", "trim", "glass", "head", "brake"]:
        sel = [t for t in tris if t[3] == k]
        if sel:
            add_part(k, np.array([t[0] for t in sel]), np.array([t[1] for t in sel]), np.array([t[2] for t in sel]))
    for z in range(2):
        sel = [t for t in tris if t[3] == "detach:%d" % z]
        if sel:
            P = np.array([t[0] for t in sel])
            c = (P.reshape(-1, 3).min(0) + P.reshape(-1, 3).max(0)) / 2
            add_part("detach", P, np.array([t[1] for t in sel]), np.array([t[2] for t in sel]), center=c, zone=z)
    for i, w in enumerate(wheels):
        add_part("wheel%d" % i, w["P"] - origin, w["N"], w["UV"], center=w["c"])
    # сглаженные нормали, AO и грязь
    bake_parts(parts, float(np.mean([w["r"] for w in wheels])))
    # бинарный файл
    blobs, meta, off = [], [], 0
    for p in parts:
        n = p["P"].shape[0] * 3
        v = np.concatenate([p["P"].reshape(n, 3), p["N"].reshape(n, 3), p["UV"].reshape(n, 2), p["UV1"].reshape(n, 2), p["AD"].reshape(n, 2)], axis=1).astype("<f4")
        b = v.tobytes()
        allp = p["P"].reshape(-1, 3)
        meta.append({"name": p["name"], "zone": p["zone"], "center": p["center"].round(4).tolist(), "verts": n, "offset": off,
                     "half": ((allp.max(0) - allp.min(0)) / 2).round(4).tolist()})
        blobs.append(b)
        off += len(b)
    head = json.dumps({"id": gid, "parts": meta}).encode()
    with open(os.path.join(OUT, gid + ".cbm"), "wb") as f:
        f.write(b"CBM2" + struct.pack("<I", len(head)) + head + b"".join(blobs))
    return {
        "id": gid, "file": fname,
        "wheels": [w["c"].round(3).tolist() for w in wheels],
        "wheel_r": [round(float(wheels[0]["r"]), 3), round(float(wheels[2]["r"]), 3)],
        "min": mn.round(3).tolist(), "max": mx.round(3).tolist(), "roof_y": round(float(roof_y), 3),
        "tris": sum(p["P"].shape[0] for p in parts),
        "parts": [(m["name"], m["verts"] // 3) for m in meta],
    }


def main():
    src_dir = sys.argv[1] if len(sys.argv) > 1 else os.path.join(ROOT, "assets", "kenney", "car")
    cmap = os.path.join(src_dir, "Textures", "colormap.png")
    os.makedirs(OUT, exist_ok=True)
    Image.open(cmap).save(os.path.join(OUT, "colormap.png"))
    paint_colormap(cmap, os.path.join(OUT, "colormap_paint.png"))
    rows = [convert(*c, src_dir) for c in CARS]
    v3 = lambda a: "Vec3::new(%.3f, %.3f, %.3f)" % tuple(a)
    rs = ["//! Сгенерировано tools/cars/convert.py из Kenney Car Kit (CC0). Не править вручную.",
          "//! Габариты моделей в метрах, координаты машины: перед -Z, высота от оси колёс.",
          "use bevy::prelude::*;", "",
          "pub struct CarFit {", "    pub id: &'static str,", "    /// центры колёс модели: перед-лево, перед-право, зад-лево, зад-право",
          "    pub wheels: [Vec3; 4],", "    /// радиус колёс модели: перед, зад", "    pub wheel_r: [f32; 2],",
          "    pub min: Vec3,", "    pub max: Vec3,", "    pub roof_y: f32,", "}", "",
          "pub const FITS: &[CarFit] = &["]
    for r in rows:
        rs.append("    CarFit { id: %s, wheels: [%s], wheel_r: [%.3f, %.3f], min: %s, max: %s, roof_y: %.3f }," % (
            json.dumps(r["id"]), ", ".join(v3(w) for w in r["wheels"]), r["wheel_r"][0], r["wheel_r"][1], v3(r["min"]), v3(r["max"]), r["roof_y"]))
    rs += ["];", "", "pub fn fit(id: &str) -> Option<&'static CarFit> {", "    FITS.iter().find(|f| f.id == id)", "}", ""]
    open(os.path.join(ROOT, "src", "carfit.rs"), "w").write("\n".join(rs))
    for r in rows:
        d = np.array(r["max"]) - np.array(r["min"])
        print(f"{r['id']:9s} {r['file']:16s} {d[0]:.2f} x {d[1]:.2f} x {d[2]:.2f} м, колёса r {r['wheel_r']}, треуг. {r['tris']}, части {r['parts']}")


if __name__ == "__main__":
    main()
