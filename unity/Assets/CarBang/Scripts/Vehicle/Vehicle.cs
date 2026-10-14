// Аркадная машина на лучевой подвеске (вместо WheelCollider): пружина и демпфер на каждое колесо,
// шина по углу увода с кругом трения, автомат, нитро, ассисты. Игрок и ИИ подают одинаковый VehicleInput.
using UnityEngine;

namespace CarBang
{
    public class Wheel
    {
        public Vector3 mount;          // точка крепления в координатах машины (верх хода подвески)
        public Transform pivot;        // визуальный узел колеса (поворот и вращение)
        public float radius;
        public bool front, left;
        // состояние
        public bool grounded;
        public float compression, prevCompression;
        public Vector3 hitPoint, hitNormal;
        public float spinDeg;
        public float load;
        public float slip;             // доля проскальзывания 0..1 для звука и следов
        public bool punctured;
        public float oilTimer;
    }

    [RequireComponent(typeof(Rigidbody))]
    public class Vehicle : MonoBehaviour
    {
        public CarSpec spec;
        public Wheel[] wheels;
        public VehicleInput input;
        /// <summary>До сигнала старта: ручник и нейтраль, ввод не применяется.</summary>
        public bool locked = true;
        public bool wrecked;
        public bool isPlayer;

        // множители от урона, бонусов, ЭМИ
        public float powerMult = 1f;
        public float gripMult = 1f;
        public float maxSpeedMult = 1f;

        public float nitro = 100f;
        public const float NITRO_MAX = 100f;

        public Rigidbody Body { get; private set; }
        public float SpeedKmh => Body ? Vector3.Dot(Body.linearVelocity, transform.forward) * 3.6f : 0f;
        public float Speed => Body ? Body.linearVelocity.magnitude : 0f;
        public int Gear { get; private set; }
        public bool Reverse { get; private set; }
        public float Rpm01 { get; private set; }
        public bool NitroActive { get; private set; }
        public bool Grounded { get; private set; }
        public float AirTime { get; private set; }
        public float SteerAngle { get; private set; }
        public float Load01 { get; private set; }

        float steerSmooth;
        float brakeHold;
        float shiftTimer;
        float upsideTimer;
        float sinceStart;
        readonly RaycastHit[] hits = new RaycastHit[8];
        Collider[] ownColliders;

        static readonly float[] GearTop = { 0f, 0.18f, 0.34f, 0.52f, 0.74f, 1.01f };

        void Awake()
        {
            Body = GetComponent<Rigidbody>();
        }

        public void Init(CarSpec s, Wheel[] w)
        {
            spec = s;
            wheels = w;
            Body = GetComponent<Rigidbody>();
            Body.mass = s.mass;
            Body.linearDamping = 0.02f;
            Body.angularDamping = 0.6f;
            Body.interpolation = RigidbodyInterpolation.Interpolate;
            Body.collisionDetectionMode = CollisionDetectionMode.ContinuousDynamic;
            Body.maxAngularVelocity = 12f;
            // низкий центр масс: чуть ниже осей колёс, чтобы машина не кувыркалась на ровном месте
            Body.centerOfMass = new Vector3(0f, -0.05f, 0.05f);
            ownColliders = GetComponentsInChildren<Collider>();
        }

        public void ResetTo(Vector3 pos, Quaternion rot)
        {
            Body.linearVelocity = Vector3.zero;
            Body.angularVelocity = Vector3.zero;
            Body.position = pos;
            Body.rotation = rot;
            transform.SetPositionAndRotation(pos, rot);
            upsideTimer = 0f;
        }

        bool IsOwn(Collider c)
        {
            for (int i = 0; i < ownColliders.Length; i++) if (ownColliders[i] == c) return true;
            return false;
        }

        void FixedUpdate()
        {
            if (wheels == null || spec == null) return;
            float dt = Time.fixedDeltaTime;
            if (!locked) sinceStart += dt;
            var up = transform.up;
            var v = Body.linearVelocity;
            float vLongBody = Vector3.Dot(v, transform.forward);
            float speed = v.magnitude;

            // --- ввод с правилами старта и заднего хода (как в прототипе)
            float thr = 0f, brk = 0f, steerIn = 0f;
            bool hand = true;
            if (!locked && !wrecked)
            {
                thr = input.Throttle;
                brk = input.Brake;
                steerIn = Mathf.Clamp(input.Steer, -1f, 1f);
                hand = input.Handbrake;
                brakeHold = brk > 0.5f ? brakeHold + dt : 0f;
                if (!Reverse && sinceStart > 0.5f && Mathf.Abs(vLongBody) < 1.4f && brakeHold > 0.3f) Reverse = true;
                if (Reverse && thr > 0.1f) Reverse = false;
            }
            else
            {
                Reverse = false;
            }

            // --- руль: угол падает со скоростью, плавное следование вводу
            float k = Mathf.Clamp01(speed / 40f);
            float maxSteer = Mathf.Lerp(spec.steerMaxDeg, spec.steerMinDeg, k);
            steerSmooth = Mathf.MoveTowards(steerSmooth, steerIn, dt * (Mathf.Abs(steerIn) > Mathf.Abs(steerSmooth) ? 3.5f : 6f));
            SteerAngle = steerSmooth * maxSteer;

            // --- двигатель: тяга падает к максималке, нитро и множители от урона
            NitroActive = !locked && !wrecked && input.Nitro && nitro > 0f && thr > 0.1f;
            if (NitroActive) nitro = Mathf.Max(0f, nitro - 25f * dt);
            else nitro = Mathf.Min(NITRO_MAX, nitro + 3.5f * dt);
            float vmax = spec.maxSpeed * maxSpeedMult * (NitroActive ? 1.15f : 1f);
            float s01 = Mathf.Clamp01(Mathf.Abs(vLongBody) / Mathf.Max(vmax, 1f));
            float taper = 1f - Mathf.Pow(s01, 1.7f);
            float driveAccel = spec.accel * powerMult * (NitroActive ? 1.6f : 1f) * taper;
            float driveForce = 0f;
            if (Reverse) driveForce = -brk * spec.mass * 4f * (1f - Mathf.Clamp01(-vLongBody / 9f));
            else driveForce = thr * spec.mass * driveAccel;

            // коробка: передача по скорости, короткая пауза тяги при переключении вверх
            int gear = Reverse ? -1 : 1;
            if (!Reverse)
            {
                for (int i = 1; i < GearTop.Length; i++) if (s01 <= GearTop[i]) { gear = i; break; }
                if (gear > Gear && Gear > 0) shiftTimer = 0.18f;
            }
            if (shiftTimer > 0f) { shiftTimer -= dt; driveForce *= 0.25f; }
            Gear = locked ? 0 : gear;
            float lo = gear > 0 ? GearTop[gear - 1] : 0f, hi = gear > 0 ? GearTop[gear] : 0.3f;
            float rpmTarget = locked ? (thr > 0f ? 0.6f : 0.1f) : Mathf.Clamp01(Mathf.InverseLerp(lo * 0.85f, hi, s01) * 0.85f + 0.12f + thr * 0.08f);
            if (Reverse) rpmTarget = Mathf.Clamp01(Mathf.Abs(vLongBody) / 9f * 0.8f + 0.15f);
            Rpm01 = Mathf.MoveTowards(Rpm01, rpmTarget, dt * 3f);
            Load01 = thr;

            int driven = 0;
            foreach (var w in wheels) if (IsDriven(w)) driven++;

            // --- колёса
            int groundedCount = 0;
            float m4 = spec.mass / wheels.Length;
            float springK = m4 * 9.81f / (spec.suspensionRest * spec.sag);
            float dampC = 2f * spec.damping * Mathf.Sqrt(springK * m4);
            float comHeight = 0.35f;
            for (int i = 0; i < wheels.Length; i++)
            {
                var w = wheels[i];
                w.prevCompression = w.compression;
                Vector3 origin = transform.TransformPoint(w.mount);
                float len = spec.suspensionRest + w.radius;
                w.grounded = false;
                int n = Physics.RaycastNonAlloc(origin, -up, hits, len, ~0, QueryTriggerInteraction.Ignore);
                float best = float.MaxValue;
                for (int h = 0; h < n; h++)
                {
                    if (IsOwn(hits[h].collider)) continue;
                    if (hits[h].distance < best)
                    {
                        best = hits[h].distance;
                        w.hitPoint = hits[h].point;
                        w.hitNormal = hits[h].normal;
                        w.grounded = true;
                    }
                }
                if (w.oilTimer > 0f) w.oilTimer -= dt;
                if (OilSlick.Covers(origin - up * len)) w.oilTimer = 1.1f;

                if (!w.grounded)
                {
                    w.compression = 0f;
                    w.load = 0f;
                    w.slip = 0f;
                    continue;
                }
                groundedCount++;
                w.compression = Mathf.Clamp(spec.suspensionRest - (best - w.radius), 0f, spec.suspensionRest);
                float compVel = (w.compression - w.prevCompression) / dt;
                float spring = Mathf.Max(0f, springK * w.compression + dampC * compVel);
                // упор в отбойник: жёсткая добавка в последних 15% хода
                if (w.compression > spec.suspensionRest * 0.85f) spring += springK * 4f * (w.compression - spec.suspensionRest * 0.85f);
                w.load = spring;
                Body.AddForceAtPosition(up * spring, origin);

                // оси шины: вперёд с учётом поворота, в плоскости дороги
                Quaternion steerRot = w.front ? Quaternion.AngleAxis(SteerAngle, up) : Quaternion.identity;
                Vector3 fwd = Vector3.ProjectOnPlane(steerRot * transform.forward, w.hitNormal).normalized;
                Vector3 right = Vector3.Cross(w.hitNormal, fwd).normalized;
                Vector3 pv = Body.GetPointVelocity(w.hitPoint);
                float vLong = Vector3.Dot(pv, fwd);
                float vLat = Vector3.Dot(pv, right);

                float mu = spec.grip * gripMult;
                if (w.oilTimer > 0f) mu *= 0.22f;
                if (w.punctured) mu *= 0.55f;
                bool handOnThis = hand && !w.front;
                float latMu = handOnThis ? mu * 0.42f : mu;
                float maxF = mu * spring;

                // боковая сила: по углу увода с насыщением; на малой скорости гасим сползание
                float slipAngle = Mathf.Atan2(vLat, Mathf.Max(Mathf.Abs(vLong), 2.5f));
                float fLat = -latMu * spring * Mathf.Clamp(slipAngle / 0.14f, -1f, 1f);
                if (speed < 3f) fLat += -vLat * m4 / dt * 0.15f;

                // продольная: тяга, тормоз, ручник, сопротивление качению
                float fLong = 0f;
                if (IsDriven(w) && driven > 0) fLong += driveForce / driven;
                float brakeIn = Reverse ? 0f : brk;
                if (brakeIn > 0f && vLongBody > -0.5f) fLong -= Mathf.Sign(vLong) * Mathf.Min(spec.brake * m4 * brakeIn, Mathf.Abs(vLong) * m4 / dt);
                if (handOnThis) fLong -= Mathf.Sign(vLong) * Mathf.Min(spec.brake * m4 * 1.2f, Mathf.Abs(vLong) * m4 / dt);
                if (hand && locked) fLong -= vLong * m4 / dt * 0.5f;
                fLong -= vLong * 18f;

                // круг трения
                float total = Mathf.Sqrt(fLong * fLong + fLat * fLat);
                float limit = Mathf.Max(maxF, latMu * spring);
                w.slip = 0f;
                if (total > limit && total > 0.01f)
                {
                    float s = limit / total;
                    fLong *= s;
                    fLat *= s;
                    w.slip = Mathf.Clamp01((total - limit) / (limit + 1f) + 0.3f);
                }
                w.slip = Mathf.Max(w.slip, Mathf.Clamp01(Mathf.Abs(slipAngle) / 0.35f - 0.3f));

                // точка приложения поднята к центру масс: меньше опрокидывающего момента
                Vector3 at = w.hitPoint + up * comHeight;
                Body.AddForceAtPosition(fwd * fLong + right * fLat, at);

                // пробитое колесо тянет в сторону
                if (w.punctured && speed > 5f) Body.AddForceAtPosition(-right * (w.left ? -1f : 1f) * spring * 0.04f, at);
                w.spinDeg += vLong / w.radius * Mathf.Rad2Deg * dt;
            }
            Grounded = groundedCount > 0;
            AirTime = Grounded ? 0f : AirTime + dt;

            // стабилизаторы: разница сжатия слева и справа на каждой оси
            AntiRoll(0, 1, springK);
            AntiRoll(2, 3, springK);

            // прижимная сила и аэродинамика
            Body.AddForce(-up * speed * speed * 0.9f);
            Body.AddForce(-v * speed * 0.35f);

            // ассист: гасим рыскание, когда игрок не рулит (машина не уходит в бесконечный занос)
            if (groundedCount >= 3 && Mathf.Abs(steerIn) < 0.1f && !hand)
            {
                float yaw = Vector3.Dot(Body.angularVelocity, up);
                Body.AddTorque(-up * yaw * spec.mass * 0.6f);
            }
            // в воздухе: выравнивание и немного управления
            if (groundedCount == 0)
            {
                Vector3 axis = Vector3.Cross(up, Vector3.up);
                Body.AddTorque(axis * spec.mass * 2.2f);
                Body.AddTorque(transform.right * (thr - brk) * spec.mass * 0.6f + transform.forward * -steerIn * spec.mass * 0.5f);
            }

            // перевернулся и стоит: сам встаёт через 2 с
            if (Vector3.Dot(up, Vector3.up) < 0.25f && speed < 3f && !wrecked)
            {
                upsideTimer += dt;
                if (upsideTimer > 2f) Flip();
            }
            else upsideTimer = 0f;
        }

        void AntiRoll(int a, int b, float springK)
        {
            if (wheels.Length <= b) return;
            var wa = wheels[a];
            var wb = wheels[b];
            float diff = (wa.compression - wb.compression) * springK * spec.antiRoll;
            if (wa.grounded) Body.AddForceAtPosition(transform.up * diff, transform.TransformPoint(wa.mount));
            if (wb.grounded) Body.AddForceAtPosition(-transform.up * diff, transform.TransformPoint(wb.mount));
        }

        bool IsDriven(Wheel w)
        {
            switch (spec.drive)
            {
                case Drive.Front: return w.front;
                case Drive.Rear: return !w.front;
                default: return true;
            }
        }

        public void Flip()
        {
            Vector3 fwd = Vector3.ProjectOnPlane(transform.forward, Vector3.up);
            if (fwd.sqrMagnitude < 0.01f) fwd = Vector3.forward;
            ResetTo(transform.position + Vector3.up * 1.6f, Quaternion.LookRotation(fwd.normalized, Vector3.up));
        }

        void LateUpdate()
        {
            if (wheels == null) return;
            foreach (var w in wheels)
            {
                if (w.pivot == null) continue;
                float ext = w.grounded ? spec.suspensionRest - w.compression : spec.suspensionRest;
                var p = w.mount - Vector3.up * ext;
                w.pivot.localPosition = p;
                float steer = w.front ? SteerAngle : 0f;
                w.pivot.localRotation = Quaternion.AngleAxis(steer, Vector3.up) * Quaternion.AngleAxis(w.spinDeg, Vector3.right);
            }
        }
    }
}
