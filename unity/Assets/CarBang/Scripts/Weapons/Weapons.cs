// Оружие демо: пулемёт на крыше (пробивает шины) и масло в задний слот.
using UnityEngine;

namespace CarBang
{
    public class MachineGun : MonoBehaviour
    {
        public bool trigger;           // держат ли гашетку (игрок или ИИ)
        public float heat;             // 0..1, при перегреве пауза
        public bool overheated;
        public Vehicle target;         // подсказка наведения (ИИ)
        public float spread = 1.2f;    // разброс, градусы
        public float damagePerHit = 1.0f;

        Vehicle veh;
        CarHealth health;
        Transform muzzle;
        float cooldown;
        LineRenderer[] tracers;
        float[] tracerLife;
        int nextTracer;
        readonly RaycastHit[] hits = new RaycastHit[8];
        const float RANGE = 120f;
        const float RATE = 12f;

        public void Init(Vehicle v, Vector3 localMuzzle)
        {
            veh = v;
            health = v.GetComponent<CarHealth>();
            muzzle = new GameObject("Muzzle").transform;
            muzzle.SetParent(transform, false);
            muzzle.localPosition = localMuzzle;
            // корпус пулемёта: вороненая сталь
            var gunMat = Mats.Lit("gun_steel", new Color(0.16f, 0.16f, 0.18f), 0.55f, 0.85f);
            var baseGo = GameObject.CreatePrimitive(PrimitiveType.Cylinder);
            Destroy(baseGo.GetComponent<Collider>());
            baseGo.transform.SetParent(transform, false);
            baseGo.transform.localPosition = localMuzzle + new Vector3(0f, -0.12f, -0.55f);
            baseGo.transform.localScale = new Vector3(0.45f, 0.06f, 0.45f);
            baseGo.GetComponent<Renderer>().sharedMaterial = gunMat;
            for (int s = -1; s <= 1; s += 2)
            {
                var barrel = GameObject.CreatePrimitive(PrimitiveType.Cylinder);
                Destroy(barrel.GetComponent<Collider>());
                barrel.transform.SetParent(transform, false);
                barrel.transform.localPosition = localMuzzle + new Vector3(s * 0.09f, 0f, -0.35f);
                barrel.transform.localRotation = Quaternion.Euler(90f, 0f, 0f);
                barrel.transform.localScale = new Vector3(0.07f, 0.4f, 0.07f);
                barrel.GetComponent<Renderer>().sharedMaterial = gunMat;
            }
            var tracerMat = Mats.Particle("tracer", Mats.SoftDot("hotdot", 3.5f), true);
            tracers = new LineRenderer[10];
            tracerLife = new float[10];
            for (int i = 0; i < tracers.Length; i++)
            {
                var go = new GameObject("Tracer");
                go.transform.SetParent(transform, false);
                var lr = go.AddComponent<LineRenderer>();
                lr.useWorldSpace = true;
                lr.positionCount = 2;
                lr.widthMultiplier = 0.06f;
                lr.sharedMaterial = tracerMat;
                lr.startColor = new Color(1f, 0.85f, 0.4f, 1f);
                lr.endColor = new Color(1f, 0.6f, 0.2f, 0f);
                lr.shadowCastingMode = UnityEngine.Rendering.ShadowCastingMode.Off;
                lr.enabled = false;
                tracers[i] = lr;
            }
        }

        void Update()
        {
            if (veh == null) return;
            float dt = Time.deltaTime;
            for (int i = 0; i < tracers.Length; i++)
            {
                if (tracerLife[i] <= 0f) continue;
                tracerLife[i] -= dt;
                if (tracerLife[i] <= 0f) tracers[i].enabled = false;
            }
            cooldown -= dt;
            bool firing = trigger && !overheated && !veh.wrecked && !veh.locked;
            if (!firing) heat = Mathf.Max(0f, heat - dt * 0.45f);
            if (overheated && heat <= 0.15f) overheated = false;
            if (firing && cooldown <= 0f)
            {
                cooldown = 1f / RATE;
                heat += 0.022f;
                if (heat >= 1f) { heat = 1f; overheated = true; }
                Shoot();
            }
        }

        /// <summary>Мягкое автонаведение: ближайшая цель в конусе 9° впереди.</summary>
        Vehicle AutoAim()
        {
            Vehicle best = null;
            float bestAng = 9f;
            foreach (var v in Arena.Cars)
            {
                if (v == null || v == veh || v.wrecked) continue;
                var d = v.Body.worldCenterOfMass - muzzle.position;
                if (d.magnitude > RANGE) continue;
                float ang = Vector3.Angle(transform.forward, d);
                if (ang < bestAng) { bestAng = ang; best = v; }
            }
            return best;
        }

        void Shoot()
        {
            var aim = target != null ? target : AutoAim();
            Vector3 dir = transform.forward;
            if (aim != null)
            {
                // упреждение по скорости цели
                var to = aim.Body.worldCenterOfMass - muzzle.position;
                float tFly = to.magnitude / 320f;
                to += aim.Body.linearVelocity * tFly;
                dir = Vector3.Slerp(dir, to.normalized, 0.85f);
            }
            dir = Quaternion.Euler(Random.Range(-spread, spread), Random.Range(-spread, spread), 0f) * dir;
            Vector3 end = muzzle.position + dir * RANGE;
            int n = Physics.RaycastNonAlloc(muzzle.position, dir, hits, RANGE, ~0, QueryTriggerInteraction.Ignore);
            float best = float.MaxValue;
            RaycastHit hit = default;
            bool any = false;
            for (int i = 0; i < n; i++)
            {
                if (hits[i].rigidbody == veh.Body) continue;
                if (hits[i].distance < best) { best = hits[i].distance; hit = hits[i]; any = true; }
            }
            if (any)
            {
                end = hit.point;
                Fx.Sparks(hit.point, hit.normal, 4, 6f);
                var victim = hit.rigidbody != null ? hit.rigidbody.GetComponent<Vehicle>() : null;
                if (victim != null)
                {
                    float mult = health != null && health.doubleDamage > 0f ? 2f : 1f;
                    victim.GetComponent<CarHealth>().TakeDamage(damagePerHit * mult, veh, hit.point, dir, true);
                    if (health != null) health.damageDealt += damagePerHit * mult;
                    // попадание рядом с колесом: шанс пробить шину
                    foreach (var w in victim.wheels)
                    {
                        if (w.punctured || w.pivot == null) continue;
                        if (Vector3.Distance(w.pivot.position, hit.point) < w.radius * 1.4f && Random.value < 0.18f)
                        {
                            w.punctured = true;
                            GameEvents.RaiseNode(victim, "Пробита шина");
                        }
                    }
                }
                else if (hit.rigidbody != null)
                {
                    hit.rigidbody.AddForceAtPosition(dir * 40f, hit.point, ForceMode.Impulse);
                    var barrel = hit.rigidbody.GetComponent<ExplosiveBarrel>();
                    if (barrel != null && Random.value < 0.25f) barrel.Ignite(0.3f, veh);
                }
            }
            var lr = tracers[nextTracer];
            lr.SetPosition(0, muzzle.position);
            lr.SetPosition(1, Vector3.Lerp(muzzle.position, end, Random.Range(0.6f, 1f)));
            lr.enabled = true;
            tracerLife[nextTracer] = 0.05f;
            nextTracer = (nextTracer + 1) % tracers.Length;
            Fx.Flame(muzzle.position, veh.Body.linearVelocity + dir * 4f, 0.25f);
            Sfx.Gun(muzzle.position);
        }
    }

    /// <summary>Задний слот: масляное пятно за машиной. 3 заряда, по одному восстанавливается за 20 с.</summary>
    public class OilDropper : MonoBehaviour
    {
        public int charges = 3;
        public const int MAX = 3;
        public float recharge;
        public bool request;
        Vehicle veh;
        float block;

        public void Init(Vehicle v) => veh = v;

        void Update()
        {
            if (veh == null) return;
            if (block > 0f) block -= Time.deltaTime;
            if (charges < MAX)
            {
                recharge += Time.deltaTime;
                if (recharge >= 20f) { recharge = 0f; charges++; }
            }
            if (request)
            {
                request = false;
                if (charges > 0 && block <= 0f && !veh.wrecked && !veh.locked)
                {
                    charges--;
                    block = 0.6f;
                    var col = GetComponent<BoxCollider>();
                    float back = col != null ? col.size.z * 0.5f + 1.6f : 3f;
                    var p = transform.position - transform.forward * back;
                    p.y = 0f;
                    OilSlick.Spawn(p, 2.8f, 30f, veh);
                    Sfx.Oil(p);
                }
            }
        }
    }
}
