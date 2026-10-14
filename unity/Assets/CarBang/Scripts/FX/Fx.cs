// Общие эффекты: искры, клубы дыма и пыли, взрывы, обломки. Одна система частиц на тип, частицы
// выпускаются через Emit в нужной точке: дёшево и без мусора.
using UnityEngine;

namespace CarBang
{
    public static class Fx
    {
        static ParticleSystem sparks, puffs, fire, chunks, flash;
        static Transform root;

        public static void Init()
        {
            root = new GameObject("FX").transform;
            var soft = Mats.SoftDot();
            var hard = Mats.SoftDot("hotdot", 3.5f);
            sparks = Make("Sparks", Mats.Particle("p_spark", hard, true), 2000, ParticleSystemRenderMode.Stretch);
            var sr = sparks.GetComponent<ParticleSystemRenderer>();
            sr.velocityScale = 0.04f;
            sr.lengthScale = 1f;
            var sm = sparks.main;
            sm.gravityModifier = 1.6f;
            var col = sparks.collision;
            col.enabled = true;
            col.type = ParticleSystemCollisionType.World;
            col.bounce = 0.35f;
            col.dampen = 0.4f;
            col.lifetimeLoss = 0.3f;
            col.quality = ParticleSystemCollisionQuality.Low;

            puffs = Make("Puffs", Mats.Particle("p_smoke", soft, false), 1500, ParticleSystemRenderMode.Billboard);
            var pm = puffs.main;
            pm.gravityModifier = -0.05f;
            var psz = puffs.sizeOverLifetime;
            psz.enabled = true;
            psz.size = new ParticleSystem.MinMaxCurve(1f, AnimationCurve.Linear(0, 0.5f, 1, 2.2f));
            var pc = puffs.colorOverLifetime;
            pc.enabled = true;
            pc.color = Fade(0.9f);
            var rot = puffs.rotationOverLifetime;
            rot.enabled = true;
            rot.z = new ParticleSystem.MinMaxCurve(-0.6f, 0.6f);

            fire = Make("Fire", Mats.Particle("p_fire", soft, true), 1000, ParticleSystemRenderMode.Billboard);
            var fsz = fire.sizeOverLifetime;
            fsz.enabled = true;
            fsz.size = new ParticleSystem.MinMaxCurve(1f, AnimationCurve.Linear(0, 1f, 1, 0.2f));
            var fc = fire.colorOverLifetime;
            fc.enabled = true;
            var g = new Gradient();
            g.SetKeys(new[] { new GradientColorKey(new Color(1f, 0.9f, 0.5f), 0f), new GradientColorKey(new Color(1f, 0.45f, 0.1f), 0.4f), new GradientColorKey(new Color(0.4f, 0.1f, 0.05f), 1f) },
                      new[] { new GradientAlphaKey(1f, 0f), new GradientAlphaKey(0.8f, 0.5f), new GradientAlphaKey(0f, 1f) });
            fc.color = g;

            chunks = Make("Chunks", Mats.Lit("chunk", new Color(0.22f, 0.21f, 0.2f), 0.3f, 0.5f), 400, ParticleSystemRenderMode.Mesh);
            var cr = chunks.GetComponent<ParticleSystemRenderer>();
            var cube = GameObject.CreatePrimitive(PrimitiveType.Cube);
            cr.mesh = cube.GetComponent<MeshFilter>().sharedMesh;
            Object.Destroy(cube);
            var cm = chunks.main;
            cm.gravityModifier = 1.4f;
            cm.startSize3D = true;
            cm.startRotation3D = true;
            var ccol = chunks.collision;
            ccol.enabled = true;
            ccol.type = ParticleSystemCollisionType.World;
            ccol.bounce = 0.3f;
            ccol.dampen = 0.3f;
            ccol.quality = ParticleSystemCollisionQuality.Low;
            var crot = chunks.rotationOverLifetime;
            crot.enabled = true;
            crot.separateAxes = true;
            crot.x = new ParticleSystem.MinMaxCurve(-6f, 6f);
            crot.y = new ParticleSystem.MinMaxCurve(-6f, 6f);
            crot.z = new ParticleSystem.MinMaxCurve(-6f, 6f);

            flash = Make("Flash", Mats.Particle("p_flash", soft, true), 50, ParticleSystemRenderMode.Billboard);
        }

        static ParticleSystem Make(string name, Material mat, int max, ParticleSystemRenderMode mode)
        {
            var go = new GameObject(name);
            go.transform.SetParent(root, false);
            var ps = go.AddComponent<ParticleSystem>();
            ps.Stop(true, ParticleSystemStopBehavior.StopEmittingAndClear);
            var main = ps.main;
            main.loop = false;
            main.playOnAwake = false;
            main.maxParticles = max;
            main.simulationSpace = ParticleSystemSimulationSpace.World;
            main.scalingMode = ParticleSystemScalingMode.Hierarchy;
            var em = ps.emission;
            em.enabled = false;
            var sh = ps.shape;
            sh.enabled = false;
            var r = go.GetComponent<ParticleSystemRenderer>();
            r.renderMode = mode;
            r.sharedMaterial = mat;
            r.shadowCastingMode = mode == ParticleSystemRenderMode.Mesh ? UnityEngine.Rendering.ShadowCastingMode.On : UnityEngine.Rendering.ShadowCastingMode.Off;
            r.sortMode = ParticleSystemSortMode.Distance;
            ps.Play();
            return ps;
        }

        public static Gradient Fade(float a0)
        {
            var g = new Gradient();
            g.SetKeys(new[] { new GradientColorKey(Color.white, 0f), new GradientColorKey(Color.white, 1f) },
                      new[] { new GradientAlphaKey(0f, 0f), new GradientAlphaKey(a0, 0.12f), new GradientAlphaKey(0f, 1f) });
            return g;
        }

        public static void Sparks(Vector3 pos, Vector3 normal, int count, float speed = 9f)
        {
            if (sparks == null) return;
            var ep = new ParticleSystem.EmitParams();
            for (int i = 0; i < count; i++)
            {
                ep.position = pos;
                ep.velocity = (normal + Random.insideUnitSphere * 0.9f).normalized * speed * Random.Range(0.4f, 1.2f);
                ep.startLifetime = Random.Range(0.25f, 0.7f);
                ep.startSize = Random.Range(0.04f, 0.08f);
                ep.startColor = Color.Lerp(new Color(1f, 0.85f, 0.4f), new Color(1f, 0.5f, 0.15f), Random.value);
                sparks.Emit(ep, 1);
            }
        }

        public static void Puff(Vector3 pos, Color color, float size, Vector3 vel, int count = 1, float life = 2.5f)
        {
            if (puffs == null) return;
            var ep = new ParticleSystem.EmitParams();
            for (int i = 0; i < count; i++)
            {
                ep.position = pos + Random.insideUnitSphere * size * 0.3f;
                ep.velocity = vel + Random.insideUnitSphere * 0.6f;
                ep.startLifetime = life * Random.Range(0.75f, 1.25f);
                ep.startSize = size * Random.Range(0.7f, 1.3f);
                ep.startColor = color;
                ep.rotation = Random.Range(0f, 360f);
                puffs.Emit(ep, 1);
            }
        }

        public static void Flame(Vector3 pos, Vector3 vel, float size)
        {
            if (fire == null) return;
            var ep = new ParticleSystem.EmitParams
            {
                position = pos + Random.insideUnitSphere * size * 0.25f,
                velocity = vel + Vector3.up * 1.6f + Random.insideUnitSphere * 0.5f,
                startLifetime = Random.Range(0.35f, 0.7f),
                startSize = size * Random.Range(0.8f, 1.2f),
                startColor = Color.white,
            };
            fire.Emit(ep, 1);
        }

        public static void Chunks(Vector3 pos, Vector3 dir, int count, Color color)
        {
            if (chunks == null) return;
            var ep = new ParticleSystem.EmitParams();
            for (int i = 0; i < count; i++)
            {
                ep.position = pos + Random.insideUnitSphere * 0.3f;
                ep.velocity = (dir + Random.insideUnitSphere * 0.8f + Vector3.up * 0.6f).normalized * Random.Range(3f, 9f);
                ep.startLifetime = Random.Range(2.5f, 5f);
                ep.startSize3D = new Vector3(Random.Range(0.06f, 0.3f), Random.Range(0.02f, 0.08f), Random.Range(0.06f, 0.25f));
                ep.startColor = color * Random.Range(0.7f, 1.1f);
                ep.rotation3D = Random.insideUnitSphere * 180f;
                chunks.Emit(ep, 1);
            }
        }

        /// <summary>Взрыв: огненный шар, дым, искры, обломки, вспышка света, толчок и урон по площади.</summary>
        public static void Explosion(Vector3 pos, float radius, float damage, Vehicle source)
        {
            var ep = new ParticleSystem.EmitParams { position = pos, startLifetime = 0.25f, startSize = radius * 2.2f, startColor = new Color(1f, 0.8f, 0.5f) };
            if (flash != null) flash.Emit(ep, 1);
            for (int i = 0; i < 28; i++) Flame(pos + Random.insideUnitSphere * radius * 0.4f, Random.insideUnitSphere * 6f, radius * 0.7f);
            Puff(pos + Vector3.up, new Color(0.12f, 0.11f, 0.1f, 1f), radius * 0.9f, Vector3.up * 2.5f, 18, 4.5f);
            Sparks(pos, Vector3.up, 60, 16f);
            Chunks(pos, Vector3.up, 14, new Color(0.25f, 0.22f, 0.2f));
            var lightGo = new GameObject("ExplosionLight");
            lightGo.transform.position = pos + Vector3.up;
            var l = lightGo.AddComponent<Light>();
            l.type = LightType.Point;
            l.color = new Color(1f, 0.6f, 0.25f);
            l.range = radius * 6f;
            l.intensity = 40f;
            lightGo.AddComponent<FadeLight>();
            Sfx.Explosion(pos);
            CameraShake.Add(pos, 1.2f);

            // толчок и урон
            var cols = Physics.OverlapSphere(pos, radius * 2.5f, ~0, QueryTriggerInteraction.Ignore);
            var done = new System.Collections.Generic.HashSet<Rigidbody>();
            foreach (var c in cols)
            {
                var rb = c.attachedRigidbody;
                if (rb == null || done.Contains(rb)) continue;
                done.Add(rb);
                rb.AddExplosionForce(rb.mass * 9f, pos, radius * 2.5f, 1.2f, ForceMode.Impulse);
                var h = rb.GetComponent<CarHealth>();
                if (h != null)
                {
                    float d = Vector3.Distance(rb.worldCenterOfMass, pos);
                    float k = Mathf.Clamp01(1f - d / (radius * 2.5f));
                    h.TakeDamage(damage * k, source, rb.worldCenterOfMass, (rb.worldCenterOfMass - pos).normalized, false);
                }
                var barrel = rb.GetComponent<ExplosiveBarrel>();
                if (barrel != null) barrel.Ignite(0.15f + Random.value * 0.2f, source);
            }
        }
    }

    /// <summary>Гасит свет вспышки за полсекунды и удаляет объект.</summary>
    public class FadeLight : MonoBehaviour
    {
        Light l;
        float i0, t;

        void Start()
        {
            l = GetComponent<Light>();
            i0 = l.intensity;
        }

        void Update()
        {
            t += Time.deltaTime;
            l.intensity = i0 * Mathf.Clamp01(1f - t / 0.5f);
            if (t > 0.5f) Destroy(gameObject);
        }
    }
}
