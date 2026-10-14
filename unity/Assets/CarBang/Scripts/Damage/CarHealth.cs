// Урон машины: по изменению скорости при ударе (Δv из импульса столкновения), броня, зоны,
// узлы (радиатор, двигатель, колесо), вмятины на модели, обломки, гибель со взрывом.
using System.Collections.Generic;
using UnityEngine;

namespace CarBang
{
    public class CarHealth : MonoBehaviour
    {
        public float hp, hpMax, armor, armorMax;
        /// <summary>Накопленный урон по зонам: перед, зад, левый борт, правый борт.</summary>
        public readonly float[] zones = new float[4];
        public bool wrecked;
        public float invulnerable;
        public float doubleDamage;
        public float damageDealt;
        public Vehicle lastAttacker;
        public float lastHitTime = -10f;

        Vehicle veh;
        Rigidbody rb;
        bool radiator, engine, wheelGone;

        class Pending
        {
            public float t0;
            public float impulse;
            public float closing;
            public Vector3 point, normal;
            public Vehicle other;
            public float otherMass;
            public bool done;
        }

        readonly Dictionary<int, Pending> pending = new Dictionary<int, Pending>();
        readonly List<int> finished = new List<int>();

        // вмятины
        class DentMesh
        {
            public MeshFilter mf;
            public Mesh mesh;
            public Vector3[] verts;
        }

        readonly List<DentMesh> dents = new List<DentMesh>();
        readonly List<Renderer> renderers = new List<Renderer>();

        public void Init(CarSpec spec, List<Transform> bodyParts)
        {
            veh = GetComponent<Vehicle>();
            rb = GetComponent<Rigidbody>();
            hpMax = hp = spec.hp;
            armorMax = armor = spec.armor;
            foreach (var t in bodyParts)
            {
                var mf = t.GetComponent<MeshFilter>();
                if (mf == null || mf.sharedMesh == null) continue;
                if (!mf.sharedMesh.isReadable)
                {
                    Debug.LogWarning("[CarBang] Меш " + mf.sharedMesh.name + " без Read/Write: вмятин не будет (меню CarBang → Переимпортировать машины)");
                    continue;
                }
                var mesh = mf.mesh; // копия на эту машину
                dents.Add(new DentMesh { mf = mf, mesh = mesh, verts = mesh.vertices });
            }
            renderers.AddRange(GetComponentsInChildren<Renderer>());
        }

        public float HpFrac => hpMax > 0f ? hp / hpMax : 0f;
        public float ArmorFrac => armorMax > 0f ? armor / armorMax : 0f;

        void Update()
        {
            if (invulnerable > 0f) invulnerable -= Time.deltaTime;
            if (doubleDamage > 0f) doubleDamage -= Time.deltaTime;
            // удар собирается за 0,15 с: импульс всех контактов этой пары
            finished.Clear();
            foreach (var kv in pending)
                if (Time.time - kv.Value.t0 > 0.15f) finished.Add(kv.Key);
            foreach (var k in finished)
            {
                Resolve(pending[k]);
                pending.Remove(k);
            }
        }

        void OnCollisionEnter(Collision c) => Collect(c, true);
        void OnCollisionStay(Collision c) => Collect(c, false);

        void Collect(Collision c, bool enter)
        {
            if (c.contactCount == 0) return;
            int key = c.rigidbody != null ? c.rigidbody.GetInstanceID() : c.collider.GetInstanceID();
            var contact = c.GetContact(0);
            if (!pending.TryGetValue(key, out var p))
            {
                if (!enter && c.impulse.magnitude / rb.mass < 1.5f) return; // скольжение вдоль стены не считаем ударом
                p = new Pending
                {
                    t0 = Time.time,
                    closing = Mathf.Abs(Vector3.Dot(c.relativeVelocity, contact.normal)),
                    point = contact.point,
                    normal = contact.normal,
                    other = c.rigidbody != null ? c.rigidbody.GetComponent<Vehicle>() : null,
                    otherMass = c.rigidbody != null ? c.rigidbody.mass : 0f,
                };
                pending[key] = p;
            }
            p.impulse += c.impulse.magnitude;
            // искры от любого заметного контакта
            float strength = c.impulse.magnitude / rb.mass;
            if (strength > 1.2f) Fx.Sparks(contact.point, contact.normal, Mathf.Clamp((int)(strength * 4f), 3, 30));
        }

        void Resolve(Pending p)
        {
            float dv = p.impulse / rb.mass;
            // Δv не больше физически возможного при этой скорости сближения
            float cap = p.otherMass > 0f ? p.closing * 2f * p.otherMass / (p.otherMass + rb.mass) * 1.1f : p.closing * 1.3f;
            dv = Mathf.Min(dv, cap);
            float dmg = DamageRules.ImpactDamage(dv);
            Sfx.Impact(p.point, Mathf.Clamp01(dv / 16f));
            if (veh.isPlayer || (p.other != null && p.other.isPlayer)) CameraShake.Add(p.point, Mathf.Clamp01(dv / 14f));
            if (dmg <= 0f) return;
            float mult = 1f;
            if (p.other != null)
            {
                mult *= p.other.spec.ramMult;
                var oh = p.other.GetComponent<CarHealth>();
                if (oh != null && oh.doubleDamage > 0f) mult *= 2f;
                if (oh != null) oh.damageDealt += dmg * mult;
            }
            else if (p.otherMass <= 0f) mult *= 0.6f; // стены и столбы бьют слабее машин
            // нормаль контакта у Unity смотрит от другого тела к этому: она же направление вмятины внутрь кузова
            TakeDamage(dmg * mult, p.other, p.point, p.normal, false, p.closing);
            if (dmg > 10f) Fx.Chunks(p.point, p.normal, Mathf.Clamp((int)(dmg / 5f), 2, 10), new Color(0.3f, 0.3f, 0.32f));
        }

        /// <summary>Урон от любого источника. dir: направление удара (куда давит).</summary>
        public void TakeDamage(float dmg, Vehicle attacker, Vector3 point, Vector3 dir, bool weapon, float impactSpeed = 0f)
        {
            if (wrecked || dmg <= 0f || invulnerable > 0f) return;
            if (attacker == veh) attacker = null;
            var r = DamageRules.ApplyHit(hp, armor, dmg, impactSpeed);
            float loss = hp - r.Hp;
            hp = r.Hp;
            armor = r.Armor;
            if (attacker != null) { lastAttacker = attacker; lastHitTime = Time.time; }

            // зона удара в координатах машины
            var lp = transform.InverseTransformPoint(point);
            int zone = Mathf.Abs(lp.z) > Mathf.Abs(lp.x) * 1.6f ? (lp.z > 0f ? 0 : 1) : (lp.x < 0f ? 2 : 3);
            zones[zone] += dmg;
            if (!weapon) Dent(point, dir, dmg);
            Nodes(zone);
            GameEvents.RaiseHit(attacker, veh, dmg, point);
            if (r.Wrecked) Wreck(attacker);
        }

        void Nodes(int zone)
        {
            if (!radiator && zones[0] >= 35f)
            {
                radiator = true;
                veh.powerMult *= 0.82f;
                GameEvents.RaiseNode(veh, "Пробит радиатор");
            }
            if (!engine && (zones[0] >= 70f || hp < 30f))
            {
                engine = true;
                veh.maxSpeedMult *= 0.85f;
                GameEvents.RaiseNode(veh, "Двигатель");
            }
            if (!wheelGone && (zone == 2 || zone == 3) && zones[zone] >= 40f)
            {
                wheelGone = true;
                foreach (var w in veh.wheels)
                    if (w.left == (zone == 2) && (w.front == (Random.value < 0.5f)))
                    {
                        w.punctured = true;
                        break;
                    }
                GameEvents.RaiseNode(veh, "Колесо");
            }
        }

        public bool RadiatorBroken => radiator;
        public bool EngineBroken => engine;

        public void Repair(float amount)
        {
            if (wrecked) return;
            hp = Mathf.Min(hpMax, hp + amount);
            armor = Mathf.Min(armorMax, armor + amount * 0.5f);
        }

        void Dent(Vector3 point, Vector3 dir, float dmg)
        {
            float depth = Mathf.Min(0.28f, dmg * 0.009f);
            if (depth < 0.01f) return;
            foreach (var d in dents)
            {
                var t = d.mf.transform;
                var lp = t.InverseTransformPoint(point);
                var ld = t.InverseTransformDirection(dir).normalized;
                float scale = Mathf.Max(1e-4f, t.lossyScale.x);
                float radius = 1.1f / scale;
                float r2 = radius * radius;
                bool changed = false;
                var v = d.verts;
                for (int i = 0; i < v.Length; i++)
                {
                    float dist2 = (v[i] - lp).sqrMagnitude;
                    if (dist2 > r2) continue;
                    float k = 1f - Mathf.Sqrt(dist2) / radius;
                    // мелкая «жевка» металла: соседние вершины мнутся по-разному
                    float jitter = 0.75f + 0.5f * Mathf.PerlinNoise(v[i].x * 9f, v[i].z * 9f + v[i].y * 5f);
                    v[i] += ld * (depth / scale) * k * k * jitter;
                    changed = true;
                }
                if (changed)
                {
                    d.mesh.vertices = v;
                    d.mesh.RecalculateNormals();
                    d.mesh.RecalculateBounds();
                }
            }
        }

        void Wreck(Vehicle by)
        {
            if (wrecked) return;
            wrecked = true;
            veh.wrecked = true;
            hp = 0f;
            // кузов обгорает
            var block = new MaterialPropertyBlock();
            foreach (var r in renderers)
            {
                if (r == null || r is ParticleSystemRenderer || r is TrailRenderer) continue;
                r.GetPropertyBlock(block);
                block.SetColor("_BaseColor", new Color(0.18f, 0.16f, 0.15f));
                r.SetPropertyBlock(block);
            }
            Fx.Explosion(rb.worldCenterOfMass, 2.2f, 0f, by);
            rb.AddForce(Vector3.up * rb.mass * 6f, ForceMode.Impulse);
            rb.AddTorque(Random.insideUnitSphere * rb.mass * 3f, ForceMode.Impulse);
            GameEvents.RaiseWrecked(veh, by);
        }
    }
}
