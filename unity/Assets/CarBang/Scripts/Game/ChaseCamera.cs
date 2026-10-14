// Камера за машиной: инерционное следование, взгляд вперёд по скорости, рывок FOV на нитро,
// тряска от ударов и взрывов, взгляд назад (V) и три дистанции (C).
using UnityEngine;

namespace CarBang
{
    public static class CameraShake
    {
        public static float Amount;

        /// <summary>Сила тряски падает с расстоянием до камеры.</summary>
        public static void Add(Vector3 at, float strength)
        {
            var cam = Camera.main;
            float k = 1f;
            if (cam != null) k = Mathf.Clamp01(1f - Vector3.Distance(cam.transform.position, at) / 60f);
            Amount = Mathf.Min(1.5f, Amount + strength * k);
        }
    }

    public class ChaseCamera : MonoBehaviour
    {
        public Vehicle target;
        int mode;
        static readonly float[] Dist = { 6.8f, 9.5f, 0.1f };
        static readonly float[] Height = { 2.3f, 3.4f, 1.15f };
        Vector3 vel;
        Camera cam;
        float fov = 62f;
        float noiseT;

        void Start()
        {
            cam = GetComponent<Camera>();
        }

        void LateUpdate()
        {
            if (target == null) return;
            if (GameInput.Pressed(Btn.Camera)) mode = (mode + 1) % Dist.Length;
            float dt = Time.unscaledDeltaTime;
            var t = target.transform;
            var rb = target.Body;
            var v = rb.linearVelocity;
            bool back = GameInput.Held(Btn.LookBack);

            // направление камеры: курс машины, при заносе и полёте сглажено скоростью
            Vector3 heading = Vector3.ProjectOnPlane(t.forward, Vector3.up).normalized;
            Vector3 vflat = Vector3.ProjectOnPlane(v, Vector3.up);
            if (vflat.magnitude > 6f && Vector3.Dot(vflat.normalized, heading) > 0.2f) heading = Vector3.Slerp(heading, vflat.normalized, 0.35f).normalized;
            if (target.wrecked) heading = Vector3.ProjectOnPlane(transform.forward, Vector3.up).normalized;
            if (back) heading = -heading;

            Vector3 desired;
            Vector3 look;
            if (mode == 2 && !back && !target.wrecked)
            {
                // вид с капота
                desired = t.TransformPoint(new Vector3(0f, Height[2], 0.6f));
                look = desired + t.forward * 20f;
                transform.position = desired;
            }
            else
            {
                float dist = Dist[mode == 2 ? 0 : mode] + Mathf.Clamp(vflat.magnitude * 0.04f, 0f, 1.8f);
                desired = t.position - heading * dist + Vector3.up * Height[mode == 2 ? 0 : mode];
                // камера не уходит под землю и за стену: подтягиваем к машине
                var from = t.position + Vector3.up * 1.2f;
                if (Physics.SphereCast(from, 0.3f, (desired - from).normalized, out var hit, Vector3.Distance(from, desired), ~0, QueryTriggerInteraction.Ignore)
                    && hit.rigidbody == null)
                    desired = hit.point + hit.normal * 0.4f;
                transform.position = Vector3.SmoothDamp(transform.position, desired, ref vel, 0.12f, Mathf.Infinity, dt);
                look = t.position + Vector3.up * 1.1f + heading * 4f;
            }
            var rot = Quaternion.LookRotation(look - transform.position, Vector3.up);

            // тряска
            noiseT += dt * 25f;
            float a = CameraShake.Amount;
            if (a > 0.001f)
            {
                rot *= Quaternion.Euler((Mathf.PerlinNoise(noiseT, 0f) - 0.5f) * 6f * a, (Mathf.PerlinNoise(0f, noiseT) - 0.5f) * 6f * a, (Mathf.PerlinNoise(noiseT, noiseT) - 0.5f) * 3f * a);
                CameraShake.Amount = Mathf.MoveTowards(a, 0f, dt * 2.5f);
            }
            transform.rotation = rot;

            // FOV: шире на скорости и с нитро
            float targetFov = 60f + Mathf.Clamp01(vflat.magnitude / 45f) * 8f + (target.NitroActive ? 9f : 0f);
            fov = Mathf.Lerp(fov, targetFov, dt * 4f);
            if (cam != null) cam.fieldOfView = fov;
        }
    }
}
