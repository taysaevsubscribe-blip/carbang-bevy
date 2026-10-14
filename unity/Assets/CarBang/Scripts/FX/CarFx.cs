// Эффекты машины: дым и огонь по урону, пар из радиатора, пламя нитро, следы шин, пыль.
using UnityEngine;

namespace CarBang
{
    public class CarFx : MonoBehaviour
    {
        Vehicle veh;
        CarHealth health;
        TrailRenderer[] skids;
        Transform[] skidAnchors;
        float smokeAcc, fireAcc, steamAcc, nitroAcc, dustAcc;
        Vector3 hood, exhaust;

        public void Init(Vehicle v, CarHealth h)
        {
            veh = v;
            health = h;
            var col = GetComponent<BoxCollider>();
            var c = col != null ? col.center : Vector3.up * 0.6f;
            var s = col != null ? col.size : new Vector3(1.8f, 1f, 4f);
            hood = new Vector3(0f, c.y + s.y * 0.35f, c.z + s.z * 0.38f);
            exhaust = new Vector3(s.x * 0.28f, c.y - s.y * 0.3f, c.z - s.z * 0.5f);

            var skidMat = Mats.LitTransparent("skid", new Color(0.03f, 0.03f, 0.03f, 0.55f), 0.1f);
            skids = new TrailRenderer[veh.wheels.Length];
            skidAnchors = new Transform[veh.wheels.Length];
            for (int i = 0; i < skids.Length; i++)
            {
                var go = new GameObject("Skid" + i);
                skidAnchors[i] = go.transform;
                var tr = go.AddComponent<TrailRenderer>();
                tr.time = 12f;
                tr.minVertexDistance = 0.25f;
                tr.widthMultiplier = 0.26f;
                tr.alignment = LineAlignment.TransformZ;
                tr.sharedMaterial = skidMat;
                tr.shadowCastingMode = UnityEngine.Rendering.ShadowCastingMode.Off;
                tr.receiveShadows = true;
                tr.emitting = false;
                tr.numCapVertices = 0;
                skids[i] = tr;
            }
        }

        void OnDestroy()
        {
            if (skidAnchors == null) return;
            foreach (var a in skidAnchors) if (a != null) Destroy(a.gameObject);
        }

        void LateUpdate()
        {
            if (veh == null) return;
            float dt = Time.deltaTime;
            // следы шин лежат на дороге: плоскость следа повёрнута нормалью вверх
            for (int i = 0; i < skids.Length; i++)
            {
                var w = veh.wheels[i];
                bool mark = w.grounded && w.slip > 0.45f && veh.Speed > 4f;
                if (w.grounded)
                {
                    skidAnchors[i].position = w.hitPoint + w.hitNormal * 0.025f;
                    skidAnchors[i].rotation = Quaternion.LookRotation(w.hitNormal, transform.forward);
                }
                skids[i].emitting = mark;
                if (mark)
                {
                    dustAcc += dt * 18f * w.slip;
                    if (dustAcc > 1f)
                    {
                        dustAcc -= 1f;
                        Fx.Puff(w.hitPoint + Vector3.up * 0.2f, new Color(0.75f, 0.72f, 0.68f, 0.55f), 0.9f, Vector3.up * 0.4f - transform.forward * 1.5f, 1, 1.6f);
                    }
                }
            }

            // дым из-под капота по урону, огонь при почти разбитой машине и после гибели
            float frac = health.HpFrac;
            var hoodW = transform.TransformPoint(hood);
            var drift = Vector3.up * 1.2f - veh.Body.linearVelocity * 0.25f;
            if (frac < 0.5f || health.wrecked)
            {
                smokeAcc += dt * (health.wrecked ? 14f : Mathf.Lerp(3f, 12f, 1f - frac * 2f));
                var col = frac < 0.25f || health.wrecked ? new Color(0.08f, 0.08f, 0.08f, 0.9f) : new Color(0.45f, 0.45f, 0.45f, 0.7f);
                while (smokeAcc > 1f) { smokeAcc -= 1f; Fx.Puff(hoodW, col, 1.1f, drift, 1, 2.8f); }
            }
            if (frac < 0.2f || health.wrecked)
            {
                fireAcc += dt * 30f;
                while (fireAcc > 1f) { fireAcc -= 1f; Fx.Flame(hoodW + Random.insideUnitSphere * 0.3f, veh.Body.linearVelocity * 0.6f, 0.7f); }
            }
            if (health.RadiatorBroken && !health.wrecked)
            {
                steamAcc += dt * 8f;
                while (steamAcc > 1f) { steamAcc -= 1f; Fx.Puff(hoodW + transform.forward * 0.4f, new Color(0.95f, 0.95f, 0.95f, 0.5f), 0.7f, drift + Vector3.up, 1, 1.4f); }
            }
            if (veh.NitroActive)
            {
                nitroAcc += dt * 60f;
                var ex = transform.TransformPoint(exhaust);
                while (nitroAcc > 1f) { nitroAcc -= 1f; Fx.Flame(ex, -transform.forward * 6f + veh.Body.linearVelocity, 0.35f); }
            }
        }
    }
}
