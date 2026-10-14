"""Минимальный читатель GLB (Kenney Car Kit): узлы, меши, атрибуты в numpy."""
import json, struct
import numpy as np

CT = {5120: np.int8, 5121: np.uint8, 5122: np.int16, 5123: np.uint16, 5125: np.uint32, 5126: np.float32}
NC = {"SCALAR": 1, "VEC2": 2, "VEC3": 3, "VEC4": 4, "MAT4": 16}


def load(path):
    b = open(path, "rb").read()
    jl = struct.unpack("<I", b[12:16])[0]
    j = json.loads(b[20:20 + jl])
    off = 20 + jl
    bl = struct.unpack("<I", b[off:off + 4])[0]
    binb = b[off + 8:off + 8 + bl]
    return j, binb


def accessor(j, binb, i):
    a = j["accessors"][i]
    bv = j["bufferViews"][a["bufferView"]]
    dt = CT[a["componentType"]]
    n = NC[a["type"]]
    start = bv.get("byteOffset", 0) + a.get("byteOffset", 0)
    stride = bv.get("byteStride", 0)
    item = np.dtype(dt).itemsize * n
    if stride and stride != item:
        raw = np.frombuffer(binb, dtype=np.uint8, count=stride * a["count"], offset=start).reshape(a["count"], stride)[:, :item]
        arr = np.frombuffer(raw.tobytes(), dtype=dt).reshape(a["count"], n)
    else:
        arr = np.frombuffer(binb, dtype=dt, count=a["count"] * n, offset=start).reshape(a["count"], n)
    return arr.astype(np.float64) if dt == np.float32 else arr.astype(np.int64)


def quat_mat(q):
    x, y, z, w = q
    return np.array([
        [1 - 2 * (y * y + z * z), 2 * (x * y - z * w), 2 * (x * z + y * w)],
        [2 * (x * y + z * w), 1 - 2 * (x * x + z * z), 2 * (y * z - x * w)],
        [2 * (x * z - y * w), 2 * (y * z + x * w), 1 - 2 * (x * x + y * y)],
    ])


def node_matrix(n):
    m = np.eye(4)
    if "matrix" in n:
        return np.array(n["matrix"]).reshape(4, 4).T
    s = np.diag(n.get("scale", [1, 1, 1]) + [1])
    r = np.eye(4)
    r[:3, :3] = quat_mat(n.get("rotation", [0, 0, 0, 1]))
    t = np.eye(4)
    t[:3, 3] = n.get("translation", [0, 0, 0])
    return t @ r @ s


def mesh_nodes(path):
    """[(имя узла, мировая матрица, [примитивы {pos,nrm,uv,idx}])]"""
    j, binb = load(path)
    parent = {}
    for i, n in enumerate(j["nodes"]):
        for c in n.get("children", []):
            parent[c] = i
    out = []
    for i, n in enumerate(j["nodes"]):
        if "mesh" not in n:
            continue
        m = node_matrix(n)
        p = parent.get(i)
        names = [n.get("name", "")]
        while p is not None:
            m = node_matrix(j["nodes"][p]) @ m
            names.append(j["nodes"][p].get("name", ""))
            p = parent.get(p)
        prims = []
        for pr in j["meshes"][n["mesh"]]["primitives"]:
            at = pr["attributes"]
            prims.append({
                "pos": accessor(j, binb, at["POSITION"]),
                "nrm": accessor(j, binb, at["NORMAL"]) if "NORMAL" in at else None,
                "uv": accessor(j, binb, at["TEXCOORD_0"]) if "TEXCOORD_0" in at else None,
                "idx": accessor(j, binb, pr["indices"]).reshape(-1) if "indices" in pr else None,
            })
        out.append((n.get("name", ""), m, prims, names))
    return out
