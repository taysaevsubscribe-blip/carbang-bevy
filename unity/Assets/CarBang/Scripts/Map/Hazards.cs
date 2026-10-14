// Ловушки и бонусы: масляные пятна, взрывные бочки, ящики бонусов.
using System.Collections.Generic;
using UnityEngine;

namespace CarBang
{
    /// <summary>Пятно масла: колёса на нём теряют сцепление. Реестр пятен общий, проверка по расстоянию.</summary>
    public class OilSlick : MonoBehaviour
    {
        public static readonly List<OilSlick> All = new List<OilSlick>();
        public float radius = 3f;
        public float life = -1f;       // < 0: навсегда
        public Vehicle owner;
        float age;
        Renderer rend;
        readonly HashSet<Vehicle> reported = new HashSet<Vehicle>();

        public static OilSlick Spawn(Vector3 pos, float radius, float life, Vehicle owner)
        {
            var go = GameObject.CreatePrimitive(PrimitiveType.Quad);
            Object.Destroy(go.GetComponent<Collider>());
            go.name = "Oil";
            go.transform.position = pos + Vector3.up * 0.04f;
            go.transform.rotation = Quaternion.Euler(90f, Random.Range(0f, 360f), 0f);
            go.transform.localScale = Vector3.one * radius * 2.3f;
            var m = Mats.LitTransparent("oil", Color.white, 0.92f);
            m.SetTexture("_BaseMap", Mats.OilBlot());
            go.GetComponent<Renderer>().sharedMaterial = m;
            go.GetComponent<Renderer>().shadowCastingMode = UnityEngine.Rendering.ShadowCastingMode.Off;
            var s = go.AddComponent<OilSlick>();
            s.radius = radius;
            s.life = life;
            s.owner = owner;
            return s;
        }

        void OnEnable() => All.Add(this);
        void OnDisable() => All.Remove(this);

        void Start() => rend = GetComponent<Renderer>();

        void Update()
        {
            if (life < 0f) return;
            age += Time.deltaTime;
            if (age > life) Destroy(gameObject);
            else if (age > life - 2f) transform.localScale = Vector3.one * radius * 2.3f * Mathf.Clamp01((life - age) / 2f);
        }

        public static bool Covers(Vector3 p)
        {
            for (int i = 0; i < All.Count; i++)
            {
                var s = All[i];
                var d = s.transform.position - p;
                if (Mathf.Abs(d.y) < 1.5f && d.x * d.x + d.z * d.z < s.radius * s.radius) return true;
            }
            return false;
        }

        /// <summary>Есть ли масло впереди на пути (для ИИ: заметил ли ловушку).</summary>
        public static OilSlick Ahead(Vector3 from, Vector3 dir, float dist, float halfWidth)
        {
            foreach (var s in All)
            {
                var d = s.transform.position - from;
                d.y = 0f;
                float along = Vector3.Dot(d, dir);
                if (along < 0f || along > dist) continue;
                float side = (d - dir * along).magnitude;
                if (side < s.radius + halfWidth) return s;
            }
            return null;
        }

        void LateUpdate()
        {
            // сообщаем о машинах, попавших в масло (для реплик и подсказок)
            foreach (var v in Arena.Cars)
            {
                if (v == null || v.wrecked) continue;
                bool on = false;
                foreach (var w in v.wheels) if (w.oilTimer > 1.0f) on = true;
                var d = v.transform.position - transform.position;
                d.y = 0f;
                if (on && d.magnitude < radius + 1.5f)
                {
                    if (reported.Add(v))
                    {
                        Sfx.Oil(v.transform.position);
                        GameEvents.RaiseOil(v);
                    }
                }
                else if (d.magnitude > radius + 6f) reported.Remove(v);
            }
        }
    }

    /// <summary>Взрывная бочка: от сильного удара или взрыва рядом детонирует через короткий запал.</summary>
    public class ExplosiveBarrel : MonoBehaviour
    {
        float fuse = -1f;
        Vehicle igniter;
        bool exploded;

        public void Ignite(float delay, Vehicle by)
        {
            if (fuse >= 0f || exploded) return;
            fuse = delay;
            igniter = by;
        }

        void OnCollisionEnter(Collision c)
        {
            if (c.impulse.magnitude > 260f)
            {
                var v = c.rigidbody != null ? c.rigidbody.GetComponent<Vehicle>() : null;
                Ignite(0.25f, v);
            }
        }

        void Update()
        {
            if (fuse < 0f || exploded) return;
            fuse -= Time.deltaTime;
            if (Random.value < 0.5f) Fx.Flame(transform.position + Vector3.up * 0.6f, Vector3.zero, 0.4f);
            if (fuse <= 0f)
            {
                exploded = true;
                Fx.Explosion(transform.position + Vector3.up * 0.5f, 3f, 42f, igniter);
                Destroy(gameObject);
            }
        }
    }

    /// <summary>Ящик бонуса: крутится, светится, возрождается через 18 с.</summary>
    public class Pickup : MonoBehaviour
    {
        public PickupKind kind;
        public static readonly List<Pickup> All = new List<Pickup>();
        float respawn;
        Transform visual;
        Light glow;

        public bool Available => respawn <= 0f;

        public static Pickup Spawn(Vector3 pos, PickupKind kind)
        {
            var go = new GameObject("Pickup_" + kind);
            go.transform.position = pos;
            var p = go.AddComponent<Pickup>();
            p.kind = kind;
            Color c = kind == PickupKind.Repair ? new Color(0.2f, 1f, 0.35f) : kind == PickupKind.Nitro ? new Color(0.2f, 0.6f, 1f) : new Color(1f, 0.25f, 0.15f);
            var box = GameObject.CreatePrimitive(PrimitiveType.Cube);
            Object.Destroy(box.GetComponent<Collider>());
            box.transform.SetParent(go.transform, false);
            box.transform.localPosition = Vector3.up * 1f;
            box.transform.localScale = Vector3.one * 1.1f;
            box.GetComponent<Renderer>().sharedMaterial = Mats.Emissive("pickup_" + kind, c, 2.2f);
            p.visual = box.transform;
            var l = new GameObject("Glow").AddComponent<Light>();
            l.transform.SetParent(go.transform, false);
            l.transform.localPosition = Vector3.up * 1.2f;
            l.type = LightType.Point;
            l.color = c;
            l.range = 7f;
            l.intensity = 3f;
            l.shadows = LightShadows.None;
            p.glow = l;
            var trig = go.AddComponent<SphereCollider>();
            trig.isTrigger = true;
            trig.radius = 2.2f;
            trig.center = Vector3.up;
            return p;
        }

        void OnEnable() => All.Add(this);
        void OnDisable() => All.Remove(this);

        void Update()
        {
            if (respawn > 0f)
            {
                respawn -= Time.deltaTime;
                if (respawn <= 0f) { visual.gameObject.SetActive(true); glow.enabled = true; }
                return;
            }
            visual.localRotation = Quaternion.Euler(20f, Time.time * 90f, 20f);
            visual.localPosition = Vector3.up * (1f + Mathf.Sin(Time.time * 2.5f) * 0.15f);
        }

        void OnTriggerEnter(Collider other)
        {
            if (!Available) return;
            var v = other.attachedRigidbody != null ? other.attachedRigidbody.GetComponent<Vehicle>() : null;
            if (v == null || v.wrecked) return;
            var h = v.GetComponent<CarHealth>();
            switch (kind)
            {
                case PickupKind.Repair: h.Repair(35f); break;
                case PickupKind.Nitro: v.nitro = Vehicle.NITRO_MAX; break;
                case PickupKind.DoubleDamage: h.doubleDamage = 10f; break;
            }
            respawn = 18f;
            visual.gameObject.SetActive(false);
            glow.enabled = false;
            Sfx.Pickup(transform.position);
            GameEvents.RaisePicked(v, kind);
        }
    }
}
