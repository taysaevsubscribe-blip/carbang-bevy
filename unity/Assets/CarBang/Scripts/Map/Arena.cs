// Арена «Свалка на закате»: кольцо 2×105×62 м шириной 16 м, срезка через центр с горбом-трамплином,
// бетонные отбойники, масло в поворотах, взрывные бочки, бонусы, кучи разбитых машин за оградой.
using System.Collections.Generic;
using UnityEngine;

namespace CarBang
{
    public static class Arena
    {
        public const float A = 105f, B = 62f, W = 16f;
        public static readonly List<Vehicle> Cars = new List<Vehicle>();
        static Transform root;

        public static TrackInfo Build()
        {
            Cars.Clear();
            root = new GameObject("Arena").transform;
            var track = new TrackInfo(A, B, W, 96);
            TrackInfo.Current = track;

            Ground();
            Road(track);
            Barriers();
            Shortcut(track);
            Hazards();
            PickupsAndProps();
            Scenery();
            return track;
        }

        static GameObject Obj(string name, Mesh mesh, Material m, Vector3 pos, Quaternion rot, bool collider, bool shadows = true)
        {
            var go = new GameObject(name);
            go.transform.SetParent(root, false);
            go.transform.SetPositionAndRotation(pos, rot);
            go.AddComponent<MeshFilter>().sharedMesh = mesh;
            var r = go.AddComponent<MeshRenderer>();
            r.sharedMaterial = m;
            r.shadowCastingMode = shadows ? UnityEngine.Rendering.ShadowCastingMode.On : UnityEngine.Rendering.ShadowCastingMode.Off;
            if (collider) go.AddComponent<MeshCollider>().sharedMesh = mesh;
            return go;
        }

        static Mesh cubeMesh;

        static Mesh Cube()
        {
            if (cubeMesh == null)
            {
                var g = GameObject.CreatePrimitive(PrimitiveType.Cube);
                cubeMesh = g.GetComponent<MeshFilter>().sharedMesh;
                Object.Destroy(g);
            }
            return cubeMesh;
        }

        /// <summary>Ящик с коллайдером: стены, блоки, трамплины.</summary>
        static GameObject Box(string name, Vector3 pos, Quaternion rot, Vector3 size, Material m)
        {
            var go = GameObject.CreatePrimitive(PrimitiveType.Cube);
            go.name = name;
            go.transform.SetParent(root, false);
            go.transform.SetPositionAndRotation(pos, rot);
            go.transform.localScale = size;
            go.GetComponent<Renderer>().sharedMaterial = m;
            return go;
        }

        static void Ground()
        {
            var m = Mats.Lit("dirt", new Color(1f, 0.95f, 0.9f), 0.12f, 0f, Mats.Dirt(), new Vector2(60f, 60f));
            var g = Box("Ground", new Vector3(0f, -0.5f, 0f), Quaternion.identity, new Vector3(700f, 1f, 700f), m);
            var pm = new PhysicsMaterial("ground") { dynamicFriction = 0.6f, staticFriction = 0.7f };
            g.GetComponent<Collider>().sharedMaterial = pm;
        }

        /// <summary>Лента дороги по эллипсу: только визуал, едем по коллайдеру земли (без ступенек на кромке).</summary>
        static void Road(TrackInfo t)
        {
            int n = 256;
            var verts = new List<Vector3>();
            var uvs = new List<Vector2>();
            var norms = new List<Vector3>();
            var tris = new List<int>();
            float dist = 0f;
            Vector3 prev = t.Point(0f);
            for (int i = 0; i <= n; i++)
            {
                float th = i / (float)n * Mathf.PI * 2f;
                var c = t.Point(th);
                dist += Vector3.Distance(c, prev);
                prev = c;
                var tan = new Vector3(-A * Mathf.Sin(th), 0f, B * Mathf.Cos(th)).normalized;
                var outward = Vector3.Cross(tan, Vector3.up).normalized * -1f; // наружу эллипса
                if (Vector3.Dot(outward, c) < 0f) outward = -outward;
                verts.Add(c - outward * W * 0.5f + Vector3.up * 0.02f);
                verts.Add(c + outward * W * 0.5f + Vector3.up * 0.02f);
                norms.Add(Vector3.up);
                norms.Add(Vector3.up);
                uvs.Add(new Vector2(0f, dist / 14f));
                uvs.Add(new Vector2(1f, dist / 14f));
                if (i < n)
                {
                    int k = i * 2;
                    tris.AddRange(new[] { k, k + 2, k + 1, k + 1, k + 2, k + 3 });
                }
            }
            var mesh = new Mesh { name = "RoadRing" };
            mesh.SetVertices(verts);
            mesh.SetNormals(norms);
            mesh.SetUVs(0, uvs);
            mesh.SetTriangles(tris, 0);
            mesh.RecalculateBounds();
            mesh.RecalculateTangents();
            // если треугольники смотрят вниз, разворачиваем (зависит от направления обхода)
            if (Vector3.Dot(Vector3.Cross(verts[2] - verts[0], verts[1] - verts[0]), Vector3.up) < 0f)
            {
                for (int i = 0; i < tris.Count; i += 3) { int x = tris[i + 1]; tris[i + 1] = tris[i + 2]; tris[i + 2] = x; }
                mesh.SetTriangles(tris, 0);
            }
            var m = Mats.Lit("road", Color.white, 0.32f, 0f, Mats.Road());
            var go = Obj("Road", mesh, m, Vector3.zero, Quaternion.identity, false, false);
            go.GetComponent<MeshRenderer>().receiveShadows = true;
        }

        static void Barriers()
        {
            var concrete = Mats.Lit("concrete", Color.white, 0.2f, 0f, Mats.Concrete(), new Vector2(2f, 1f));
            var hazard = Mats.Lit("hazard", Color.white, 0.35f, 0f, Mats.Hazard(), new Vector2(3f, 0.5f));
            int n = 120;
            for (int ring = 0; ring < 2; ring++)
            {
                float off = ring == 0 ? W * 0.5f + 0.9f : -(W * 0.5f + 0.9f);
                for (int i = 0; i < n; i++)
                {
                    float t0 = i / (float)n * Mathf.PI * 2f, t1 = (i + 1) / (float)n * Mathf.PI * 2f;
                    var p0 = Offset(t0, off);
                    var p1 = Offset(t1, off);
                    var mid = (p0 + p1) * 0.5f;
                    // проёмы внутреннего отбойника: въезды на срезку и в поле
                    if (ring == 1 && Mathf.Abs(mid.x) < 7f) continue;
                    if (ring == 1 && Mathf.Abs(mid.z) < 7f) continue;
                    var dir = p1 - p0;
                    float len = dir.magnitude + 0.15f;
                    var rot = Quaternion.LookRotation(dir.normalized, Vector3.up);
                    var b = Box("Barrier", mid + Vector3.up * 0.55f, rot, new Vector3(0.7f, 1.1f, len), concrete);
                    b.isStatic = true;
                    var top = Box("BarrierTop", mid + Vector3.up * 1.12f, rot, new Vector3(0.72f, 0.06f, len), hazard);
                    Object.Destroy(top.GetComponent<Collider>());
                    top.isStatic = true;
                }
            }
        }

        static Vector3 Offset(float th, float off)
        {
            var c = new Vector3(A * Mathf.Cos(th), 0f, B * Mathf.Sin(th));
            var tan = new Vector3(-A * Mathf.Sin(th), 0f, B * Mathf.Cos(th)).normalized;
            var outward = new Vector3(tan.z, 0f, -tan.x);
            if (Vector3.Dot(outward, c) < 0f) outward = -outward;
            return c + outward * off;
        }

        /// <summary>Срезка поперёк поля (вдоль оси Z) с горбом посередине: на скорости машина прыгает.</summary>
        static void Shortcut(TrackInfo t)
        {
            float z0 = -(B - W * 0.5f), z1 = B - W * 0.5f;
            var road = Mats.Lit("road_short", Color.white, 0.3f, 0f, Mats.Road(), new Vector2(1f, 7f));
            var strip = Box("ShortcutRoad", new Vector3(0f, 0.015f, 0f), Quaternion.identity, new Vector3(10f, 0.03f, z1 - z0), road);
            Object.Destroy(strip.GetComponent<Collider>());
            // горб: подъём 9 м до 1,8 м, площадка 3 м, спуск
            var hz = Mats.Lit("hazard_ramp", Color.white, 0.4f, 0f, Mats.Hazard(), new Vector2(2f, 2f));
            float h = 1.8f, run = 9f, top = 3f;
            float ang = Mathf.Atan2(h, run) * Mathf.Rad2Deg;
            float slopeLen = Mathf.Sqrt(h * h + run * run);
            Box("RampUp", new Vector3(0f, h * 0.5f - 0.25f, -top * 0.5f - run * 0.5f), Quaternion.Euler(-ang, 0f, 0f), new Vector3(9f, 0.5f, slopeLen), hz);
            Box("RampTop", new Vector3(0f, h - 0.25f, 0f), Quaternion.identity, new Vector3(9f, 0.5f, top + 0.4f), hz);
            Box("RampDown", new Vector3(0f, h * 0.5f - 0.25f, top * 0.5f + run * 0.5f), Quaternion.Euler(ang, 0f, 0f), new Vector3(9f, 0.5f, slopeLen), hz);
            // узлы графа ИИ: южный и северный въезды
            int south = t.NearestRing(new Vector3(0f, 0f, -B));
            int north = t.NearestRing(new Vector3(0f, 0f, B));
            t.AddShortcut(south, north, 8);
            // поперечная срезка по оси X через поле, без покрытия
            int east = t.NearestRing(new Vector3(A, 0f, 0f));
            int west = t.NearestRing(new Vector3(-A, 0f, 0f));
            t.AddShortcut(east, west, 12);
        }

        static void Hazards()
        {
            // масло в поворотах на траектории: ловушка для торопливых
            OilSlick.Spawn(Offset(Mathf.PI * 0.12f, -2f), 3.2f, -1f, null);
            OilSlick.Spawn(Offset(Mathf.PI * 1.12f, -2f), 3.2f, -1f, null);
            OilSlick.Spawn(new Vector3(0f, 0f, 30f), 2.6f, -1f, null);
            // бочки: у въездов на срезку и у стен
            BarrelCluster(Offset(Mathf.PI * 0.5f, W * 0.5f - 1.5f));
            BarrelCluster(Offset(Mathf.PI * 1.5f, W * 0.5f - 1.5f));
            BarrelCluster(Offset(Mathf.PI * 0.85f, -(W * 0.5f - 1.5f)));
            BarrelCluster(Offset(Mathf.PI * 1.85f, -(W * 0.5f - 1.5f)));
            BarrelCluster(new Vector3(-30f, 0f, 0f));
            BarrelCluster(new Vector3(30f, 0f, 0f));
        }

        static void BarrelCluster(Vector3 c)
        {
            var red = Mats.Lit("barrel", new Color(0.75f, 0.1f, 0.06f), 0.45f, 0.35f);
            var band = Mats.Lit("barrel_band", new Color(0.9f, 0.75f, 0.1f), 0.4f, 0.3f);
            for (int i = 0; i < 3; i++)
            {
                var p = c + new Vector3(Mathf.Cos(i * 2.1f) * 0.9f, 0.6f, Mathf.Sin(i * 2.1f) * 0.9f);
                var go = GameObject.CreatePrimitive(PrimitiveType.Cylinder);
                go.name = "Barrel";
                go.transform.SetParent(root, false);
                go.transform.position = p;
                go.transform.localScale = new Vector3(0.75f, 0.6f, 0.75f);
                go.GetComponent<Renderer>().sharedMaterial = red;
                Object.Destroy(go.GetComponent<Collider>());
                var cc = go.AddComponent<CapsuleCollider>();
                cc.height = 2f;
                cc.radius = 0.5f;
                var b = GameObject.CreatePrimitive(PrimitiveType.Cylinder);
                Object.Destroy(b.GetComponent<Collider>());
                b.transform.SetParent(go.transform, false);
                b.transform.localScale = new Vector3(1.02f, 0.08f, 1.02f);
                b.transform.localPosition = new Vector3(0f, 0.45f, 0f);
                b.GetComponent<Renderer>().sharedMaterial = band;
                var rb = go.AddComponent<Rigidbody>();
                rb.mass = 60f;
                go.AddComponent<ExplosiveBarrel>();
            }
        }

        static void PickupsAndProps()
        {
            Pickup.Spawn(Offset(Mathf.PI * 0.25f, 0f), PickupKind.Repair);
            Pickup.Spawn(Offset(Mathf.PI * 1.25f, 0f), PickupKind.Repair);
            Pickup.Spawn(Offset(Mathf.PI * 0.75f, 0f), PickupKind.Nitro);
            Pickup.Spawn(Offset(Mathf.PI * 1.75f, 0f), PickupKind.Nitro);
            Pickup.Spawn(new Vector3(0f, 2.2f, 0f), PickupKind.DoubleDamage); // на горбу: награда за прыжок

            // конусы вдоль срезки и стопки ящиков в поле: разлетаются от удара
            var cone = Resources.Load<GameObject>("CarBangProps/cone");
            var crate = Resources.Load<GameObject>("CarBangProps/box");
            for (int i = -4; i <= 4; i++)
            {
                if (i == 0) continue;
                for (int s = -1; s <= 1; s += 2)
                    Prop(cone, new Vector3(s * 5.6f, 0f, i * 11f), 1.6f, 6f, new Color(1f, 0.45f, 0.1f));
            }
            for (int k = 0; k < 6; k++)
            {
                var c = new Vector3(Random.Range(-70f, 70f), 0f, Random.Range(-30f, 30f));
                if (Mathf.Abs(c.x) < 10f) c.x += 20f;
                for (int j = 0; j < 4; j++)
                    Prop(crate, c + new Vector3((j % 2) * 1.3f, (j / 2) * 1.25f + 0.6f, 0f), 1.2f, 25f, new Color(0.6f, 0.45f, 0.28f));
            }
        }

        static void Prop(GameObject prefab, Vector3 pos, float size, float mass, Color fallback)
        {
            GameObject go;
            if (prefab != null)
            {
                go = Object.Instantiate(prefab, root);
                go.transform.position = pos;
                var b = CarFactory.RendererBounds(go.transform);
                float k = size / Mathf.Max(0.01f, b.size.y);
                go.transform.localScale *= k;
                foreach (var r in go.GetComponentsInChildren<Renderer>())
                    r.sharedMaterial = Mats.Lit("kenney_prop", Color.white, 0.3f, 0f, Resources.Load<Texture2D>("CarBangCars/colormap"));
                var bb = CarFactory.RendererBounds(go.transform);
                var col = go.AddComponent<BoxCollider>();
                col.center = go.transform.InverseTransformPoint(bb.center);
                var ls = go.transform.lossyScale;
                col.size = new Vector3(bb.size.x / ls.x, bb.size.y / ls.y, bb.size.z / ls.z);
            }
            else
            {
                go = GameObject.CreatePrimitive(PrimitiveType.Cube);
                go.transform.SetParent(root, false);
                go.transform.position = pos + Vector3.up * size * 0.5f;
                go.transform.localScale = Vector3.one * size * 0.8f;
                go.GetComponent<Renderer>().sharedMaterial = Mats.Lit("prop_" + fallback, fallback, 0.3f);
            }
            var rb = go.AddComponent<Rigidbody>();
            rb.mass = mass;
            rb.Sleep();
        }

        /// <summary>За оградой: кучи разбитых машин Kenney, фонари с тёплым светом.</summary>
        static void Scenery()
        {
            string[] wrecks = { "sedan", "van", "suv", "police", "taxi", "ambulance", "delivery", "sedan-sports" };
            var rnd = new System.Random(17);
            var rust = Mats.Lit("wreck", new Color(0.55f, 0.42f, 0.35f), 0.15f, 0.2f, Resources.Load<Texture2D>("CarBangCars/colormap"));
            for (int i = 0; i < 26; i++)
            {
                float th = (float)rnd.NextDouble() * Mathf.PI * 2f;
                var p = Offset(th, W * 0.5f + 7f + (float)rnd.NextDouble() * 14f);
                var prefab = Resources.Load<GameObject>("CarBangProps/" + wrecks[rnd.Next(wrecks.Length)]);
                int stack = 1 + rnd.Next(3);
                for (int s = 0; s < stack; s++)
                {
                    GameObject go;
                    if (prefab != null)
                    {
                        go = Object.Instantiate(prefab, root);
                        go.transform.localScale *= 2.4f;
                        foreach (var r in go.GetComponentsInChildren<Renderer>()) r.sharedMaterial = rust;
                    }
                    else
                    {
                        go = Box("Wreck", Vector3.zero, Quaternion.identity, new Vector3(1.9f, 1.1f, 4f), rust);
                        Object.Destroy(go.GetComponent<Collider>());
                    }
                    go.transform.position = p + Vector3.up * (s * 1.3f);
                    go.transform.rotation = Quaternion.Euler((float)rnd.NextDouble() * 8f - 4f, (float)rnd.NextDouble() * 360f, s % 2 == 1 ? 180f : (float)rnd.NextDouble() * 10f - 5f);
                    go.isStatic = true;
                }
            }
            // фонари по внешней ограде
            var pole = Mats.Lit("pole", new Color(0.25f, 0.25f, 0.27f), 0.4f, 0.6f);
            var lamp = Mats.Emissive("lamp", new Color(1f, 0.75f, 0.45f), 6f);
            for (int i = 0; i < 12; i++)
            {
                float th = (i + 0.5f) / 12f * Mathf.PI * 2f;
                var p = Offset(th, W * 0.5f + 2.5f);
                var post = Box("LampPost", p + Vector3.up * 4f, Quaternion.identity, new Vector3(0.25f, 8f, 0.25f), pole);
                post.isStatic = true;
                var head = Box("LampHead", p + Vector3.up * 8.1f - (p.normalized * 0.9f), Quaternion.identity, new Vector3(0.6f, 0.2f, 1.6f), lamp);
                Object.Destroy(head.GetComponent<Collider>());
                var l = new GameObject("LampLight").AddComponent<Light>();
                l.transform.SetParent(root, false);
                l.transform.position = p + Vector3.up * 7.6f - p.normalized * 1.2f;
                l.type = LightType.Point;
                l.range = 22f;
                l.intensity = 9f;
                l.color = new Color(1f, 0.72f, 0.42f);
                l.shadows = LightShadows.None;
            }
        }
    }
}
