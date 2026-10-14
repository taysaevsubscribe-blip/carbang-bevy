"""Габариты моделей наборов Kenney (CC0) -> src/kitbox.rs.
Таблица нужна для коллизий: карта строится одинаково с графикой и без (сервер, тесты).
Запуск: python3 tools/kits/gen_table.py"""
import glob, os, sys
import numpy as np
ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
sys.path.insert(0, os.path.join(ROOT, "tools", "cars"))
from glb import mesh_nodes

KITS = ["city_roads", "city_commercial", "city_suburban", "city_industrial", "racing"]
rows = []
for kit in KITS:
    for f in sorted(glob.glob(os.path.join(ROOT, "assets", "kenney", kit, "*.glb"))):
        pts = []
        for name, M, prims, chain in mesh_nodes(f):
            for p in prims:
                pts.append((M[:3, :3] @ p["pos"].T).T + M[:3, 3])
        P = np.vstack(pts)
        rows.append((f"{kit}/{os.path.basename(f)[:-4]}", P.min(0), P.max(0)))
rows.sort(key=lambda r: r[0])
out = ["//! Сгенерировано tools/kits/gen_table.py. Габариты моделей Kenney (единицы модели).", "",
       "/// (путь без .glb относительно assets/kenney, min, max)",
       "pub const KIT: &[(&str, [f32; 3], [f32; 3])] = &["]
for k, a, b in rows:
    out.append('    ("%s", [%.4f, %.4f, %.4f], [%.4f, %.4f, %.4f]),' % (k, *a, *b))
out += ["];", "", "pub fn bbox(name: &str) -> Option<([f32; 3], [f32; 3])> {",
        "    KIT.binary_search_by(|r| r.0.cmp(name)).ok().map(|i| (KIT[i].1, KIT[i].2))", "}", ""]
open(os.path.join(ROOT, "src", "kitbox.rs"), "w").write("\n".join(out))
print(len(rows), "моделей")
