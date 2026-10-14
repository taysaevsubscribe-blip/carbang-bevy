"""Запекание для моделей Kenney: сглаженные нормали, затенение (AO) и грязь по вершинам.

- нормали: усреднение по соседним граням с углом меньше порога (фаски скругляются, углы коробок остаются);
- AO: доля лучей полусферы, упёршихся в кузов или землю в пределах 1.6 м;
- грязь: низ кузова, арки колёс и затенённые места.
Результат хранится в .cbm (CBM2): на вершину ещё 2 числа (ao, dirt).
"""
import numpy as np


def _weld_keys(P):
    return [tuple(k) for k in np.round(P.reshape(-1, 3) * 2000).astype(np.int64)]


def face_data(P):
    n = np.cross(P[:, 1] - P[:, 0], P[:, 2] - P[:, 0])
    a = np.linalg.norm(n, axis=1)
    return n / np.maximum(a, 1e-12)[:, None], a * 0.5


def smooth_normals(P, N_orig, angle_deg=48.0):
    """P: (T,3,3). Возвращает (T,3,3) нормали; вырожденные грани оставляют исходные."""
    T = P.shape[0]
    FN, A = face_data(P)
    keys = _weld_keys(P)
    groups = {}
    for i, k in enumerate(keys):
        groups.setdefault(k, []).append(i // 3)
    cos_t = np.cos(np.radians(angle_deg))
    out = np.array(N_orig, dtype=np.float64).reshape(T, 3, 3).copy()
    for i, k in enumerate(keys):
        t = i // 3
        if A[t] < 1e-10:
            continue
        fs = np.array(sorted(set(groups[k])))
        m = (FN[fs] @ FN[t]) > cos_t
        n = (FN[fs[m]] * A[fs[m], None]).sum(0)
        ln = np.linalg.norm(n)
        if ln > 1e-12:
            out[t, i % 3] = n / ln
    return out


def _hemisphere(n_rays, seed=7):
    rng = np.random.default_rng(seed)
    # косинусное распределение, стратифицированное
    k = int(np.sqrt(n_rays))
    u = (np.arange(k)[:, None] + rng.random((k, k))) / k
    v = (np.arange(k)[None, :] + rng.random((k, k))) / k
    r = np.sqrt(u.ravel())
    phi = 2 * np.pi * v.ravel()
    return np.stack([r * np.cos(phi), r * np.sin(phi), np.sqrt(np.maximum(0, 1 - r * r))], -1)


def _basis(n):
    a = np.array([1.0, 0, 0]) if abs(n[0]) < 0.9 else np.array([0, 1.0, 0])
    t = np.cross(n, a); t /= np.linalg.norm(t)
    return t, np.cross(n, t)


def ambient_occlusion(points, normals, occ_tris, ground_y=None, n_rays=49, max_d=1.6):
    """points/normals: (V,3). occ_tris: (T,3,3). Возвращает AO 0..1 (1 — открыто)."""
    H = _hemisphere(n_rays)
    v0 = occ_tris[:, 0]; e1 = occ_tris[:, 1] - v0; e2 = occ_tris[:, 2] - v0
    lo = occ_tris.min(1); hi = occ_tris.max(1)
    ao = np.ones(len(points))
    for i, (p, n) in enumerate(zip(points, normals)):
        t1, t2 = _basis(n)
        D = H[:, :1] * t1 + H[:, 1:2] * t2 + H[:, 2:3] * n  # (R,3)
        o = p + n * 0.01
        # отбор треугольников в радиусе
        sel = np.all((hi > o - max_d) & (lo < o + max_d), axis=1)
        hit_d = np.full(len(D), np.inf)
        if sel.any():
            V0, E1, E2 = v0[sel], e1[sel], e2[sel]
            pv = np.cross(D[:, None, :], E2[None])            # (R,T,3)
            det = np.einsum("rtk,tk->rt", pv, E1)
            inv = 1.0 / np.where(np.abs(det) < 1e-12, np.nan, det)
            s = o - V0                                          # (T,3)
            u = np.einsum("tk,rtk->rt", s, pv) * inv
            q = np.cross(s, E1)                                 # (T,3)
            w = np.einsum("rk,tk->rt", D, q) * inv
            d = np.einsum("tk,tk->t", E2, q)[None] * inv
            ok = (u >= 0) & (w >= 0) & (u + w <= 1) & (d > 1e-3) & (d < max_d)
            dd = np.where(ok, d, np.inf)
            hit_d = dd.min(1)
        if ground_y is not None:
            with np.errstate(divide="ignore", invalid="ignore"):
                dg = (ground_y - o[1]) / D[:, 1]
            dg = np.where((dg > 1e-3) & (dg < max_d), dg, np.inf)
            hit_d = np.minimum(hit_d, dg)
        # близкие препятствия затеняют сильнее
        occ = np.where(np.isfinite(hit_d), 1.0 - (hit_d / max_d) ** 2 * 0.6, 0.0)
        ao[i] = 1.0 - occ.mean()
    return np.clip(ao, 0.0, 1.0)


def bake_parts(parts, wheel_r):
    """parts: список dict с P (T,3,3, локально), N, center. Добавляет N (сглаженные) и AD (T,3,2)."""
    body = [p for p in parts if not p["name"].startswith("wheel")]
    wheels = [p for p in parts if p["name"].startswith("wheel")]
    # нормали кузова считаются вместе: стыки краски и отделки без швов
    allP = np.concatenate([p["P"] + p["center"] for p in body])
    allN = np.concatenate([p["N"] for p in body])
    nb = smooth_normals(allP, allN)
    k = 0
    for p in body:
        if p["name"] in ("head", "brake", "glass"):
            k += len(p["P"]); continue
        p["N"] = nb[k:k + len(p["P"])]
        k += len(p["P"])
    for w in wheels:
        w["N"] = smooth_normals(w["P"], w["N"], 60.0)
    wheel_world = [w["P"] + w["center"] for w in wheels]
    occ = np.concatenate([allP] + wheel_world)
    ground = -wheel_r
    for p in body:
        P = (p["P"] + p["center"]).reshape(-1, 3)
        Nn = p["N"].reshape(-1, 3)
        keys = _weld_keys(P.reshape(-1, 3, 3))
        uniq = {}
        for i, kk in enumerate(keys):
            uniq.setdefault(kk, []).append(i)
        idx = [v[0] for v in uniq.values()]
        nrm = np.array([Nn[v].mean(0) for v in uniq.values()])
        nrm /= np.maximum(np.linalg.norm(nrm, axis=1, keepdims=True), 1e-9)
        ao_u = ambient_occlusion(P[idx], nrm, occ, ground)
        ao = np.empty(len(P))
        for j, v in enumerate(uniq.values()):
            ao[v] = ao_u[j]
        y = P[:, 1]
        # грязь: ниже оси колёс сильнее, у арок колёс ещё сильнее
        low = np.clip((wheel_r * 1.3 - y) / (wheel_r * 2.0), 0, 1) ** 1.3
        dw = np.min([np.linalg.norm(P - w["center"], axis=1) for w in wheels], axis=0)
        arch = np.clip(1.0 - (dw - wheel_r) / (wheel_r * 0.9), 0, 1)
        dirt = np.clip(low * 0.75 + arch * 0.55 + (1 - ao) * 0.35, 0, 1)
        p["AD"] = np.stack([ao, dirt], -1).reshape(-1, 3, 2)
    for w in wheels:
        P = w["P"].reshape(-1, 3)
        ao = ambient_occlusion(P, w["N"].reshape(-1, 3), w["P"], None, n_rays=25, max_d=0.5)
        w["AD"] = np.stack([ao, np.full(len(P), 0.35)], -1).reshape(-1, 3, 2)
