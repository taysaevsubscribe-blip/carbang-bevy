// Сборка машины: модель Kenney Car Kit (FBX из Resources/CarBangCars) выравнивается и масштабируется
// по колёсам, колёса переносятся на узлы подвески, добавляются физика, фары, стопы, звук и урон.
// Если модели нет, машина собирается из примитивов, чтобы демо всё равно запускалось.
using System.Collections.Generic;
using UnityEngine;

namespace CarBang
{
    public static class CarFactory
    {
        public static Vehicle Build(CarSpec spec, Vector3 pos, Quaternion rot, bool player)
        {
            var root = new GameObject(spec.displayName);
            root.transform.SetPositionAndRotation(pos, rot);
            var rb = root.AddComponent<Rigidbody>();
            rb.mass = spec.mass;

            var visual = new GameObject("Visual").transform;
            visual.SetParent(root.transform, false);

            List<Transform> bodyParts;
            Transform[] wheelNodes;
            float wheelRadius;
            Bounds body;
            var prefab = Resources.Load<GameObject>("CarBangCars/" + spec.model);
            if (prefab != null && FromModel(prefab, spec, root.transform, visual, out bodyParts, out wheelNodes, out wheelRadius, out body))
            {
            }
            else
            {
                if (prefab == null) Debug.LogWarning("[CarBang] Нет модели Resources/CarBangCars/" + spec.model + ", машина из примитивов");
                FromPrimitives(spec, root.transform, visual, out bodyParts, out wheelNodes, out wheelRadius, out body);
            }

            // коллайдер кузова: чуть уже модели и приподнят над землёй, чтобы днище не цеплялось
            var col = root.AddComponent<BoxCollider>();
            float bottom = Mathf.Max(body.min.y, -wheelRadius * 0.35f);
            col.center = new Vector3(body.center.x, (bottom + body.max.y) * 0.5f + 0.05f, body.center.z);
            col.size = new Vector3(body.size.x * 0.94f, (body.max.y - bottom) * 0.92f, body.size.z * 0.97f);
            var pm = new PhysicsMaterial("car") { dynamicFriction = 0.35f, staticFriction = 0.4f, bounciness = 0.12f, frictionCombine = PhysicsMaterialCombine.Minimum, bounceCombine = PhysicsMaterialCombine.Average };
            col.sharedMaterial = pm;

            // колёса: узел подвески в координатах машины, модель колеса внутри
            var wheels = new Wheel[4];
            for (int i = 0; i < 4; i++)
            {
                var node = wheelNodes[i];
                var center = root.transform.InverseTransformPoint(RendererBounds(node).center);
                var pivot = new GameObject("WheelPivot" + i).transform;
                pivot.SetParent(root.transform, false);
                pivot.localPosition = center;
                pivot.localRotation = Quaternion.identity;
                node.SetParent(pivot, true);
                wheels[i] = new Wheel
                {
                    mount = center + Vector3.up * spec.suspensionRest * (1f - spec.sag),
                    pivot = pivot,
                    radius = wheelRadius,
                    front = center.z > 0f,
                    left = center.x < 0f,
                };
            }
            // порядок: перед-лево, перед-право, зад-лево, зад-право (нужен стабилизаторам)
            System.Array.Sort(wheels, (a, b) => Order(a).CompareTo(Order(b)));

            var veh = root.AddComponent<Vehicle>();
            veh.isPlayer = player;
            veh.Init(spec, wheels);

            AddLights(root.transform, body, spec, veh);
            var health = root.AddComponent<CarHealth>();
            health.Init(spec, bodyParts);
            var snd = root.AddComponent<EngineSound>();
            snd.Init(veh);
            root.AddComponent<CarFx>().Init(veh, health);
            return veh;
        }

        static int Order(Wheel w) => (w.front ? 0 : 2) + (w.left ? 0 : 1);

        static bool FromModel(GameObject prefab, CarSpec spec, Transform root, Transform visual, out List<Transform> bodyParts, out Transform[] wheelNodes, out float wheelRadius, out Bounds body)
        {
            bodyParts = new List<Transform>();
            wheelNodes = new Transform[4];
            wheelRadius = 0.35f;
            body = new Bounds(Vector3.zero, Vector3.one);
            var inst = Object.Instantiate(prefab, visual);
            inst.name = "Model";
            inst.transform.localPosition = Vector3.zero;

            var found = new List<Transform>();
            foreach (var t in inst.GetComponentsInChildren<Transform>(true))
            {
                string n = t.name.ToLowerInvariant();
                if (n.Contains("wheel") && (n.Contains("front") || n.Contains("back") || n.Contains("rear")) && t.GetComponentInChildren<Renderer>() != null)
                {
                    // берём верхний узел колеса, не вложенный в другое колесо
                    bool nested = false;
                    foreach (var f in found) if (t.IsChildOf(f)) nested = true;
                    if (!nested) found.Add(t);
                }
            }
            if (found.Count < 4)
            {
                Object.Destroy(inst);
                Debug.LogWarning("[CarBang] В модели " + prefab.name + " не найдено 4 колеса (wheel-front-left и т.п.)");
                return false;
            }

            // направление «вперёд» модели по колёсам: от задней оси к передней
            Vector3 fc = Vector3.zero, bc = Vector3.zero;
            int nf = 0, nb = 0;
            foreach (var w in found)
            {
                var c = RendererBounds(w).center;
                if (w.name.ToLowerInvariant().Contains("front")) { fc += c; nf++; } else { bc += c; nb++; }
            }
            if (nf == 0 || nb == 0) { Object.Destroy(inst); return false; }
            fc /= nf;
            bc /= nb;
            Vector3 fwdWorld = Vector3.ProjectOnPlane(fc - bc, root.up);
            float measuredBase = fwdWorld.magnitude;
            if (measuredBase < 1e-4f) { Object.Destroy(inst); return false; }
            Vector3 fwdLocal = root.InverseTransformDirection(fwdWorld.normalized);
            // только поворот вокруг вертикали: FromToRotation при развороте на 180° мог бы перевернуть модель
            float yaw = Vector3.SignedAngle(new Vector3(fwdLocal.x, 0f, fwdLocal.z), Vector3.forward, Vector3.up);
            visual.localRotation = Quaternion.AngleAxis(yaw, Vector3.up);
            float scale = spec.wheelbase / measuredBase;
            visual.localScale = Vector3.one * scale;

            // после поворота и масштаба: центр кузова по X/Z в ноль, оси колёс на высоту 0
            float axleY = 0f;
            foreach (var w in found) axleY += root.InverseTransformPoint(RendererBounds(w).center).y;
            axleY /= found.Count;
            wheelRadius = RendererBounds(found[0]).extents.y;
            var bodyB = BodyBounds(inst.transform, found, root);
            visual.localPosition -= new Vector3(bodyB.center.x, axleY, bodyB.center.z);
            body = BodyBounds(inst.transform, found, root);

            // колёса по позициям: перед/зад и лево/право определяются по координатам, не по именам
            var sorted = new List<Transform>(found);
            sorted.Sort((a, b) => Score(root, a).CompareTo(Score(root, b)));
            for (int i = 0; i < 4; i++) wheelNodes[i] = sorted[i];

            foreach (var r in inst.GetComponentsInChildren<MeshRenderer>(true))
            {
                bool isWheel = false;
                foreach (var w in found) if (r.transform.IsChildOf(w)) isWheel = true;
                if (!isWheel) bodyParts.Add(r.transform);
            }
            ApplyKenneyMaterials(inst, spec);
            return true;
        }

        static int Score(Transform root, Transform w)
        {
            var p = root.InverseTransformPoint(RendererBounds(w).center);
            return (p.z > 0f ? 0 : 2) + (p.x < 0f ? 0 : 1);
        }

        public static Bounds RendererBounds(Transform t)
        {
            var rs = t.GetComponentsInChildren<Renderer>();
            var b = rs.Length > 0 ? rs[0].bounds : new Bounds(t.position, Vector3.zero);
            for (int i = 1; i < rs.Length; i++) b.Encapsulate(rs[i].bounds);
            return b;
        }

        /// <summary>Габарит кузова без колёс в координатах машины.</summary>
        static Bounds BodyBounds(Transform model, List<Transform> wheels, Transform root)
        {
            bool any = false;
            var b = new Bounds();
            foreach (var r in model.GetComponentsInChildren<Renderer>())
            {
                bool isWheel = false;
                foreach (var w in wheels) if (r.transform.IsChildOf(w)) isWheel = true;
                if (isWheel) continue;
                var wb = r.bounds;
                // 8 углов мирового габарита в координаты машины
                for (int i = 0; i < 8; i++)
                {
                    var c = wb.center + Vector3.Scale(wb.extents, new Vector3((i & 1) == 0 ? -1 : 1, (i & 2) == 0 ? -1 : 1, (i & 4) == 0 ? -1 : 1));
                    var lp = root.InverseTransformPoint(c);
                    if (!any) { b = new Bounds(lp, Vector3.zero); any = true; } else b.Encapsulate(lp);
                }
            }
            return any ? b : new Bounds(Vector3.up * 0.5f, new Vector3(1.8f, 1f, 4f));
        }

        /// <summary>Материалы Kenney → URP Lit с палитрой colormap; немного металла и блеска.</summary>
        static void ApplyKenneyMaterials(GameObject inst, CarSpec spec)
        {
            var tex = Resources.Load<Texture2D>("CarBangCars/colormap");
            foreach (var r in inst.GetComponentsInChildren<Renderer>(true))
            {
                var mats = r.sharedMaterials;
                for (int i = 0; i < mats.Length; i++)
                {
                    Texture t = tex;
                    if (t == null && mats[i] != null && mats[i].HasProperty("_BaseMap")) t = mats[i].GetTexture("_BaseMap");
                    if (t == null && mats[i] != null && mats[i].HasProperty("_MainTex")) t = mats[i].GetTexture("_MainTex");
                    mats[i] = Mats.Lit("kenney_car", Color.white, 0.55f, 0.25f, t);
                }
                r.sharedMaterials = mats;
                r.shadowCastingMode = UnityEngine.Rendering.ShadowCastingMode.On;
            }
        }

        static void FromPrimitives(CarSpec spec, Transform root, Transform visual, out List<Transform> bodyParts, out Transform[] wheelNodes, out float wheelRadius, out Bounds body)
        {
            bodyParts = new List<Transform>();
            wheelNodes = new Transform[4];
            wheelRadius = 0.38f;
            float L = spec.wheelbase * 1.55f, W = 1.85f, H = 0.75f;
            var paint = Mats.Lit("prim_paint_" + spec.id, spec.accent, 0.6f, 0.3f);
            var dark = Mats.Lit("prim_dark", new Color(0.08f, 0.08f, 0.09f), 0.3f);
            var glass = Mats.Lit("prim_glass", new Color(0.05f, 0.07f, 0.09f), 0.95f, 0.5f);
            var lower = Prim(PrimitiveType.Cube, visual, new Vector3(0, H * 0.55f, 0), new Vector3(W, H, L), paint);
            var cabin = Prim(PrimitiveType.Cube, visual, new Vector3(0, H * 1.25f, -L * 0.05f), new Vector3(W * 0.86f, H * 0.7f, L * 0.48f), glass);
            bodyParts.Add(lower);
            bodyParts.Add(cabin);
            for (int i = 0; i < 4; i++)
            {
                bool front = i < 2, left = i % 2 == 0;
                var w = new GameObject("wheel").transform;
                w.SetParent(visual, false);
                w.localPosition = new Vector3((left ? -1 : 1) * W * 0.5f, 0f, (front ? 1 : -1) * spec.wheelbase * 0.5f);
                var tire = Prim(PrimitiveType.Cylinder, w, Vector3.zero, new Vector3(wheelRadius * 2f, 0.14f, wheelRadius * 2f), dark);
                tire.localRotation = Quaternion.Euler(0, 0, 90);
                wheelNodes[i] = w;
            }
            body = new Bounds(new Vector3(0, H * 0.85f, 0), new Vector3(W, H * 1.9f, L));
        }

        static Transform Prim(PrimitiveType type, Transform parent, Vector3 pos, Vector3 scale, Material m)
        {
            var go = GameObject.CreatePrimitive(type);
            Object.Destroy(go.GetComponent<Collider>());
            go.transform.SetParent(parent, false);
            go.transform.localPosition = pos;
            go.transform.localScale = scale;
            go.GetComponent<Renderer>().sharedMaterial = m;
            return go.transform;
        }

        static void AddLights(Transform root, Bounds body, CarSpec spec, Vehicle veh)
        {
            var lights = root.gameObject.AddComponent<CarLights>();
            var headMat = Mats.Emissive("headlight", new Color(1f, 0.95f, 0.8f), 4f);
            float z = body.max.z + 0.02f, y = body.min.y + body.size.y * 0.32f, x = body.extents.x * 0.62f;
            var quads = new List<Renderer>();
            for (int s = -1; s <= 1; s += 2)
            {
                var q = GameObject.CreatePrimitive(PrimitiveType.Quad);
                Object.Destroy(q.GetComponent<Collider>());
                q.transform.SetParent(root, false);
                q.transform.localPosition = new Vector3(s * x, y, z);
                q.transform.localRotation = Quaternion.Euler(0, 180, 0);
                q.transform.localScale = new Vector3(0.32f, 0.14f, 1f);
                q.GetComponent<Renderer>().sharedMaterial = headMat;
                var tail = GameObject.CreatePrimitive(PrimitiveType.Quad);
                Object.Destroy(tail.GetComponent<Collider>());
                tail.transform.SetParent(root, false);
                tail.transform.localPosition = new Vector3(s * x, y, body.min.z - 0.02f);
                tail.transform.localScale = new Vector3(0.3f, 0.12f, 1f);
                quads.Add(tail.GetComponent<Renderer>());
            }
            // два прожектора на машину, без теней: дёшево и даёт пятна света на асфальте
            var spot = new GameObject("Headlight").AddComponent<Light>();
            spot.type = LightType.Spot;
            spot.transform.SetParent(root, false);
            spot.transform.localPosition = new Vector3(0, y + 0.1f, z + 0.1f);
            spot.transform.localRotation = Quaternion.Euler(9f, 0, 0);
            spot.range = 45f;
            spot.spotAngle = 62f;
            spot.innerSpotAngle = 30f;
            spot.intensity = 18f;
            spot.color = new Color(1f, 0.92f, 0.78f);
            spot.shadows = LightShadows.None;
            lights.Init(veh, quads);

            // полоса цвета команды по борту: отличать соперника издалека
            var stripeMat = Mats.Emissive("accent_" + spec.id, spec.accent, 1.5f);
            for (int s = -1; s <= 1; s += 2)
            {
                var q = GameObject.CreatePrimitive(PrimitiveType.Quad);
                Object.Destroy(q.GetComponent<Collider>());
                q.transform.SetParent(root, false);
                q.transform.localPosition = new Vector3(s * (body.extents.x + 0.015f), body.min.y + body.size.y * 0.45f, body.center.z);
                q.transform.localRotation = Quaternion.Euler(0, s > 0 ? -90 : 90, 0);
                q.transform.localScale = new Vector3(body.size.z * 0.7f, 0.06f, 1f);
                q.GetComponent<Renderer>().sharedMaterial = stripeMat;
            }
        }
    }

    /// <summary>Стоп-сигналы: ярче при торможении, тускло при движении.</summary>
    public class CarLights : MonoBehaviour
    {
        Vehicle veh;
        List<Renderer> tails;
        Material dim, bright;

        public void Init(Vehicle v, List<Renderer> t)
        {
            veh = v;
            tails = t;
            dim = Mats.Emissive("tail_dim", new Color(1f, 0.05f, 0.03f), 0.8f);
            bright = Mats.Emissive("tail_bright", new Color(1f, 0.05f, 0.03f), 5f);
            foreach (var r in tails) r.sharedMaterial = dim;
        }

        void Update()
        {
            if (veh == null) return;
            bool braking = !veh.wrecked && (veh.input.Brake > 0.1f || veh.input.Handbrake) && !veh.Reverse;
            var m = braking ? bright : dim;
            foreach (var r in tails) if (r.sharedMaterial != m) r.sharedMaterial = m;
        }
    }
}
