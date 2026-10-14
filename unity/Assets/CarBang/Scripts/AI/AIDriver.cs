// Соперник: характер, ярость и набор тактик. Едет тем же VehicleInput, что и игрок, без читов физики.
// Охота с упреждением, таран с разгона и нитро, отъезд после удара, засада у срезки, уход на ремонт,
// бегство с маслом. Разъярённый соперник не замечает ловушек: его можно заманить в масло и на бочки.
using System.Collections.Generic;
using UnityEngine;

namespace CarBang
{
    public enum AIState { Race, Hunt, Ram, BackOff, Recover, Repair, Ambush, Evade }

    public class AIDriver : MonoBehaviour
    {
        [Header("Характер 0..1")]
        public float aggression = 0.8f;
        public float skill = 0.75f;
        public float cunning = 0.6f;
        public float vengeance = 0.8f;
        public float greed = 0.5f;

        public float rage;
        public AIState state = AIState.Hunt;
        public Vehicle target;

        Vehicle me;
        CarHealth health;
        MachineGun gun;
        OilDropper oil;
        AIVoice voice;
        TrackInfo track;

        float stateTimer, thinkTimer, ramCooldown, stuckTimer, burstTimer, oilTimer, ambushCooldown = 20f;
        float reverseSteer;
        bool firingPhase;
        List<int> path = new List<int>();
        int pathIdx;
        Vector3 goal;
        Vector3 ambushPoint, ambushLook;
        OilSlick avoiding;
        readonly HashSet<OilSlick> decided = new HashSet<OilSlick>();
        readonly HashSet<OilSlick> willAvoid = new HashSet<OilSlick>();
        readonly RaycastHit[] probeHits = new RaycastHit[6];

        public void Init(Vehicle self, Vehicle player, TrackInfo t)
        {
            me = self;
            target = player;
            track = t;
            health = GetComponent<CarHealth>();
            gun = GetComponent<MachineGun>();
            oil = GetComponent<OilDropper>();
            voice = GetComponent<AIVoice>();
            GameEvents.Hit += OnHit;
            GameEvents.SlippedOnOil += OnOil;
            GameEvents.Picked += OnPicked;
            GameEvents.Wrecked += OnWrecked;
        }

        void OnDestroy()
        {
            GameEvents.Hit -= OnHit;
            GameEvents.SlippedOnOil -= OnOil;
            GameEvents.Picked -= OnPicked;
            GameEvents.Wrecked -= OnWrecked;
        }

        // ------------------------------------------------------------------ реакции на события

        void OnHit(Vehicle attacker, Vehicle victim, float dmg, Vector3 p)
        {
            if (victim == me && attacker == target)
            {
                rage = Mathf.Clamp01(rage + dmg / 35f * (0.6f + vengeance * 0.6f));
                if (dmg > 6f) Say(Line.Hurt);
                if (state != AIState.Repair && state != AIState.Evade) SetState(AIState.Hunt);
            }
            else if (attacker == me && victim == target && dmg > 3f)
            {
                // удар достиг цели: отъехать и зайти на новый разгон
                ramCooldown = 4f;
                if (dmg > 8f) Say(health.lastAttacker == target && Time.time - health.lastHitTime < 15f ? Line.Revenge : Line.Taunt);
                SetState(AIState.BackOff);
                stateTimer = 1.1f;
                reverseSteer = Random.value < 0.5f ? -1f : 1f;
            }
            else if (victim == me && attacker == null && dmg > 20f)
            {
                Say(Line.Explosion);
            }
        }

        void OnOil(Vehicle v)
        {
            if (v == me) { Say(Line.Oil); rage = Mathf.Clamp01(rage + 0.15f); }
        }

        void OnPicked(Vehicle v, PickupKind k)
        {
            if (v == me) Say(k == PickupKind.Repair ? Line.PickRepair : k == PickupKind.Nitro ? Line.PickNitro : Line.PickDamage);
        }

        void OnWrecked(Vehicle v, Vehicle by)
        {
            if (v == me) Say(Line.Wrecked, true);
            else if (v == target) Say(Line.Victory, true);
        }

        void Say(Line l, bool force = false)
        {
            if (voice != null) voice.Say(l, force);
        }

        void SetState(AIState s)
        {
            if (state == s) return;
            state = s;
            stateTimer = 0f;
            path.Clear();
            if (s == AIState.Ambush) Say(Line.Ambush);
        }

        // ------------------------------------------------------------------ решение

        void FixedUpdate()
        {
            if (me == null || track == null) return;
            if (me.wrecked || me.locked)
            {
                me.input = new VehicleInput { Handbrake = me.locked };
                if (gun) gun.trigger = false;
                return;
            }
            float dt = Time.fixedDeltaTime;
            stateTimer += dt;
            thinkTimer -= dt;
            ramCooldown -= dt;
            ambushCooldown -= dt;
            rage = Mathf.Max(0f, rage - dt * 0.03f);
            if (thinkTimer <= 0f) { Think(); thinkTimer = 0.25f; }
            Drive(dt);
            Weapons(dt);
        }

        bool TargetAlive => target != null && !target.wrecked;

        void Think()
        {
            float hpFrac = health.HpFrac;
            float dist = TargetAlive ? Vector3.Distance(me.transform.position, target.transform.position) : 999f;

            // ремонт: при малом кузове, если ящик доступен (жадные бегут раньше)
            if (state != AIState.Repair && hpFrac < 0.3f + greed * 0.15f && NearestPickup(PickupKind.Repair) != null && rage < 0.8f)
            {
                Say(Line.LowHp);
                SetState(AIState.Repair);
                return;
            }
            switch (state)
            {
                case AIState.BackOff:
                    if (stateTimer > 1.2f) SetState(AIState.Hunt);
                    return;
                case AIState.Recover:
                    if (stateTimer > 1.3f) SetState(AIState.Hunt);
                    return;
                case AIState.Repair:
                    var pk = NearestPickup(PickupKind.Repair);
                    if (pk == null || hpFrac > 0.65f) SetState(AIState.Hunt);
                    return;
                case AIState.Ambush:
                    if (!TargetAlive || stateTimer > 18f) { SetState(AIState.Hunt); return; }
                    if (dist < 30f && Facing(target.transform.position) < 50f) { SetState(AIState.Ram); Say(Line.Taunt); }
                    return;
                case AIState.Evade:
                    if (stateTimer > 6f || dist > 45f) SetState(AIState.Hunt);
                    return;
            }
            if (!TargetAlive) { SetState(AIState.Race); return; }

            // засада: хитрый соперник иногда ждёт у въезда на срезку, куда едет игрок
            if (ambushCooldown <= 0f && dist > 60f && rage < 0.5f && Random.value < cunning * 0.5f)
            {
                ambushCooldown = 35f;
                if (PlanAmbush()) { SetState(AIState.Ambush); return; }
            }
            ambushCooldown = Mathf.Max(ambushCooldown, 0f);

            // бегство: игрок сзади и близко, кузов слабый, трусость выше ярости
            if (hpFrac < 0.4f && dist < 20f && Facing(target.transform.position) > 120f && rage < 0.5f && Random.value < 1f - aggression * 0.6f)
            {
                SetState(AIState.Evade);
                return;
            }

            if (dist < 28f && ramCooldown <= 0f && Facing(target.transform.position) < 35f) SetState(AIState.Ram);
            else if (state == AIState.Ram && (dist > 40f || ramCooldown > 0f)) SetState(AIState.Hunt);
            else if (state != AIState.Ram) SetState(AIState.Hunt);
        }

        bool PlanAmbush()
        {
            // въезды на срезку: точки кольца у осей; выбираем ту, к которой игрок едет
            var tv = target.Body.linearVelocity;
            var candidates = new[] { new Vector3(0f, 0f, -Arena.B + 2f), new Vector3(0f, 0f, Arena.B - 2f), new Vector3(Arena.A - 2f, 0f, 0f), new Vector3(-Arena.A + 2f, 0f, 0f) };
            float best = -1f;
            foreach (var c in candidates)
            {
                var toC = c - target.transform.position;
                float score = Vector3.Dot(tv.normalized, toC.normalized) * 50f - toC.magnitude * 0.3f - Vector3.Distance(me.transform.position, c) * 0.4f;
                if (Vector3.Dot(tv, toC) > 0f && score > best)
                {
                    best = score;
                    // ждём внутри поля у проёма, носом к кольцу
                    ambushPoint = c * 0.82f;
                    ambushLook = c;
                }
            }
            return best > -1f;
        }

        float Facing(Vector3 p)
        {
            var d = p - me.transform.position;
            d.y = 0f;
            return Vector3.Angle(me.transform.forward, d);
        }

        Pickup NearestPickup(PickupKind k)
        {
            Pickup best = null;
            float bd = float.MaxValue;
            foreach (var p in Pickup.All)
            {
                if (p.kind != k || !p.Available) continue;
                float d = Vector3.Distance(p.transform.position, me.transform.position);
                if (d < bd) { bd = d; best = p; }
            }
            return best;
        }

        // ------------------------------------------------------------------ вождение

        Vector3 PathPoint(Vector3 dest)
        {
            // прямо, если цель близко и видна; иначе по графу трассы
            var from = me.transform.position + Vector3.up * 0.8f;
            var to = dest + Vector3.up * 0.8f;
            if (Vector3.Distance(from, to) < 45f && !Blocked(from, to)) { path.Clear(); return dest; }
            int goalNode = track.NearestNode(dest);
            if (path.Count == 0 || path[path.Count - 1] != goalNode || pathIdx >= path.Count)
            {
                path = track.Path(track.NearestNode(me.transform.position), goalNode);
                pathIdx = 0;
            }
            while (pathIdx < path.Count - 1 && Vector3.Distance(Flat(me.transform.position), track.nodes[path[pathIdx]]) < 14f) pathIdx++;
            return pathIdx < path.Count ? track.nodes[path[pathIdx]] : dest;
        }

        static Vector3 Flat(Vector3 v) => new Vector3(v.x, 0f, v.z);

        bool Blocked(Vector3 a, Vector3 b)
        {
            int n = Physics.RaycastNonAlloc(a, (b - a).normalized, probeHits, Vector3.Distance(a, b), ~0, QueryTriggerInteraction.Ignore);
            for (int i = 0; i < n; i++) if (probeHits[i].rigidbody == null && probeHits[i].collider.name.StartsWith("Barrier")) return true;
            return false;
        }

        void Drive(float dt)
        {
            float throttle = 1f, brake = 0f;
            bool hand = false, nitro = false;
            float speed = me.Speed;
            Vector3 pos = me.transform.position;

            switch (state)
            {
                case AIState.BackOff:
                case AIState.Recover:
                    me.input = new VehicleInput { Throttle = 0f, Brake = 1f, Steer = reverseSteer, Handbrake = false };
                    return;
                case AIState.Race:
                {
                    int i = track.NearestRing(pos);
                    goal = track.ring[(i + 4) % track.ring.Count];
                    break;
                }
                case AIState.Repair:
                {
                    var pk = NearestPickup(PickupKind.Repair);
                    goal = PathPoint(pk != null ? pk.transform.position : pos);
                    break;
                }
                case AIState.Ambush:
                {
                    float d = Vector3.Distance(Flat(pos), Flat(ambushPoint));
                    if (d > 4f) goal = PathPoint(ambushPoint);
                    else
                    {
                        // стоим носом к проёму, мотор на холостых
                        goal = ambushLook;
                        throttle = 0f;
                        brake = speed > 1f ? 1f : 0f;
                        hand = speed < 1f;
                        if (Facing(ambushLook) > 25f) { throttle = 0.35f; hand = false; }
                    }
                    break;
                }
                case AIState.Evade:
                {
                    // уходим по кольцу от игрока
                    int i = track.NearestRing(pos);
                    var away = pos - target.transform.position;
                    int dir = Vector3.Dot(track.ringDir[i], away) >= 0f ? 1 : -1;
                    goal = track.ring[(i + dir * 4 + track.ring.Count) % track.ring.Count];
                    nitro = true;
                    break;
                }
                default:
                {
                    if (!TargetAlive) { goal = track.ring[(track.NearestRing(pos) + 4) % track.ring.Count]; break; }
                    // упреждение: тем дальше, чем выше мастерство
                    var tp = target.transform.position;
                    float d = Vector3.Distance(pos, tp);
                    float lead = Mathf.Clamp(d / Mathf.Max(speed, 8f), 0f, 1.4f) * skill;
                    var aim = tp + target.Body.linearVelocity * lead;
                    if (state == AIState.Ram)
                    {
                        goal = aim;
                        nitro = Facing(aim) < 12f && d > 8f && (me.nitro > 30f || rage > 0.6f);
                    }
                    else
                    {
                        // после тарана держим дистанцию: кружим сбоку, пока не пройдёт перезарядка
                        if (ramCooldown > 0f && d < 25f)
                            aim += Vector3.Cross(Vector3.up, (tp - pos).normalized) * 12f;
                        goal = PathPoint(aim);
                    }
                    break;
                }
            }

            // ловушки впереди: замечаем масло; разъярённый или неумелый едет прямо в него
            var fwdFlat = Flat(me.transform.forward).normalized;
            var slick = OilSlick.Ahead(pos, fwdFlat, 10f + speed * 0.9f, 1.2f);
            if (slick != null && slick.owner != me)
            {
                if (decided.Add(slick))
                {
                    bool notice = Random.value < skill * (1f - rage * 0.9f);
                    if (notice) willAvoid.Add(slick);
                }
                if (willAvoid.Contains(slick))
                {
                    var toS = Flat(slick.transform.position - pos);
                    var side = Vector3.Cross(Vector3.up, fwdFlat);
                    float s = Vector3.Dot(toS, side) > 0f ? -1f : 1f;
                    goal = slick.transform.position + side * s * (slick.radius + 3.5f) + fwdFlat * 4f;
                }
            }

            // руль на цель, щупы препятствий
            var toGoal = Flat(goal - pos);
            float ang = Vector3.SignedAngle(fwdFlat, toGoal.normalized, Vector3.up);
            float steer = Mathf.Clamp(ang / 28f, -1f, 1f);
            steer += Avoid(fwdFlat, speed);
            steer = Mathf.Clamp(steer, -1f, 1f);

            // скорость: тормозим перед крутым поворотом, на развороте помогаем ручником
            float absAng = Mathf.Abs(ang);
            if (absAng > 55f && speed > 14f) { throttle = 0.3f; brake = 0.6f * skill; }
            if (absAng > 75f && speed > 9f) hand = true;
            if (absAng > 110f && speed < 6f && toGoal.magnitude < 15f)
            {
                // цель позади и рядом: разворот задним ходом
                throttle = 0f;
                brake = 1f;
                steer = -Mathf.Sign(ang);
            }
            if (state == AIState.Ram) { brake = 0f; hand = false; throttle = 1f; }

            // застрял: газ есть, скорости нет
            if (throttle > 0.5f && speed < 1.5f && state != AIState.Ambush) stuckTimer += dt; else stuckTimer = 0f;
            if (stuckTimer > 1.5f)
            {
                stuckTimer = 0f;
                reverseSteer = -Mathf.Sign(steer == 0f ? 1f : steer);
                SetState(AIState.Recover);
            }
            if (Vector3.Dot(me.transform.up, Vector3.up) < 0.3f && speed < 2f) me.Flip();

            me.input = new VehicleInput { Throttle = throttle, Brake = brake, Steer = steer, Handbrake = hand, Nitro = nitro || (rage > 0.75f && absAng < 15f) };
        }

        float Avoid(Vector3 fwd, float speed)
        {
            float range = 6f + speed * 0.6f;
            var o = me.transform.position + Vector3.up * 0.6f;
            float s = 0f;
            for (int k = -1; k <= 1; k += 2)
            {
                var dir = Quaternion.AngleAxis(k * 22f, Vector3.up) * fwd;
                int n = Physics.RaycastNonAlloc(o, dir, probeHits, range, ~0, QueryTriggerInteraction.Ignore);
                for (int i = 0; i < n; i++)
                {
                    var h = probeHits[i];
                    if (h.rigidbody == me.Body) continue;
                    if (h.rigidbody != null && h.rigidbody.GetComponent<Vehicle>() == target && state == AIState.Ram) continue;
                    if (h.rigidbody != null && h.rigidbody.mass < 100f) continue; // ящики и конусы сносим
                    s -= k * (1f - h.distance / range) * 0.9f;
                    break;
                }
            }
            return s;
        }

        void Weapons(float dt)
        {
            if (gun != null)
            {
                gun.target = TargetAlive ? target : null;
                bool can = TargetAlive && Facing(target.transform.position) < 14f && Vector3.Distance(me.transform.position, target.transform.position) < 60f
                           && !Blocked(me.transform.position + Vector3.up, target.transform.position + Vector3.up);
                burstTimer -= dt;
                if (burstTimer <= 0f)
                {
                    firingPhase = !firingPhase;
                    burstTimer = firingPhase ? Random.Range(0.6f, 1.4f) * (0.5f + aggression) : Random.Range(0.4f, 1.2f) * (1.5f - aggression);
                }
                gun.trigger = firingPhase && can;
                gun.spread = Mathf.Lerp(4f, 1.2f, skill);
            }
            if (oil != null && TargetAlive)
            {
                oilTimer -= dt;
                var d = target.transform.position - me.transform.position;
                if (oilTimer <= 0f && d.magnitude < 25f && Vector3.Angle(me.transform.forward, d) > 150f && target.Speed > 8f)
                {
                    oilTimer = 1f;
                    if (Random.value < 0.25f + cunning * 0.3f) oil.request = true;
                }
            }
        }
    }
}
