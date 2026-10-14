// Звуки демо синтезируются при запуске (без аудиофайлов): удары, взрыв, выстрел, бонус, масло, сигналы старта.
// Воспроизведение через пул 3D-источников с затуханием по расстоянию.
using UnityEngine;

namespace CarBang
{
    public static class Sfx
    {
        const int SR = 44100;
        static AudioClip[] impacts;
        static AudioClip explosion, gun, pickup, oil, beep, go, horn, scrape;
        static AudioSource[] pool;
        static int next;
        static System.Random rng = new System.Random(3);

        static float N() => (float)(rng.NextDouble() * 2.0 - 1.0);

        public static void Init()
        {
            impacts = new AudioClip[3];
            for (int k = 0; k < 3; k++) impacts[k] = Make("impact" + k, 0.6f, (t, i) =>
            {
                float thump = Mathf.Sin(t * 6.2832f * (55f + k * 12f) * (1f - t)) * Mathf.Exp(-t * 9f);
                float metal = (Mathf.Sin(t * 6.2832f * (420f + k * 90f)) * 0.5f + Mathf.Sin(t * 6.2832f * (1130f + k * 170f)) * 0.3f) * Mathf.Exp(-t * 14f);
                float crunch = N() * Mathf.Exp(-t * 22f);
                return Mathf.Clamp(thump * 0.9f + metal * 0.45f + crunch * 0.6f, -1f, 1f);
            });
            float lp = 0f;
            explosion = Make("explosion", 2.4f, (t, i) =>
            {
                lp += (N() - lp) * 0.06f;
                float boom = Mathf.Sin(t * 6.2832f * 38f * (1f - t * 0.3f)) * Mathf.Exp(-t * 3f);
                float crackle = (rng.NextDouble() < 0.002 * Mathf.Exp(-t * 2f) ? N() * 0.8f : 0f);
                return Mathf.Clamp((lp * 3.2f + boom * 0.8f) * Mathf.Exp(-t * 1.6f) * Mathf.Min(1f, t * 80f) + crackle, -1f, 1f);
            });
            gun = Make("gun", 0.09f, (t, i) => Mathf.Clamp((N() * 0.8f + Mathf.Sin(t * 6.2832f * 140f) * 0.6f) * Mathf.Exp(-t * 45f), -1f, 1f));
            pickup = Make("pickup", 0.45f, (t, i) =>
            {
                float f = t < 0.12f ? 660f : t < 0.24f ? 880f : 1320f;
                return Mathf.Sin(t * 6.2832f * f) * 0.5f * Mathf.Exp(-((t % 0.12f)) * 6f) * (1f - t / 0.45f);
            });
            float lp2 = 0f;
            oil = Make("oil", 0.5f, (t, i) => { lp2 += (N() - lp2) * 0.2f; return lp2 * 1.6f * Mathf.Exp(-t * 7f); });
            beep = Make("beep", 0.18f, (t, i) => Mathf.Sin(t * 6.2832f * 660f) * 0.5f * Mathf.Min(1f, (0.18f - t) * 40f));
            go = Make("go", 0.5f, (t, i) => Mathf.Sin(t * 6.2832f * 990f) * 0.5f * Mathf.Min(1f, (0.5f - t) * 10f));
            horn = Make("horn", 0.6f, (t, i) => (Mathf.Sign(Mathf.Sin(t * 6.2832f * 370f)) * 0.25f + Mathf.Sign(Mathf.Sin(t * 6.2832f * 466f)) * 0.25f) * Mathf.Min(1f, (0.6f - t) * 20f));
            float lp3 = 0f;
            scrape = Make("scrape", 0.4f, (t, i) => { lp3 += (N() - lp3) * 0.5f; return lp3 * 0.7f * (1f - t / 0.4f); });

            var root = new GameObject("SfxPool").transform;
            pool = new AudioSource[24];
            for (int i = 0; i < pool.Length; i++)
            {
                var go2 = new GameObject("sfx" + i);
                go2.transform.SetParent(root, false);
                var s = go2.AddComponent<AudioSource>();
                s.playOnAwake = false;
                s.spatialBlend = 1f;
                s.rolloffMode = AudioRolloffMode.Logarithmic;
                s.minDistance = 6f;
                s.maxDistance = 400f;
                s.dopplerLevel = 0.3f;
                pool[i] = s;
            }
        }

        delegate float Gen(float t, int i);

        static AudioClip Make(string name, float seconds, Gen g)
        {
            int n = (int)(seconds * SR);
            var data = new float[n];
            for (int i = 0; i < n; i++) data[i] = g(i / (float)SR, i);
            var c = AudioClip.Create(name, n, 1, SR, false);
            c.SetData(data, 0);
            return c;
        }

        static void Play(AudioClip clip, Vector3 pos, float vol, float pitch, bool spatial = true, float minDist = 6f)
        {
            if (pool == null || clip == null) return;
            var s = pool[next];
            next = (next + 1) % pool.Length;
            s.transform.position = pos;
            s.spatialBlend = spatial ? 1f : 0f;
            s.minDistance = minDist;
            s.pitch = pitch;
            s.volume = vol;
            s.clip = clip;
            s.Play();
        }

        public static void Impact(Vector3 p, float strength)
        {
            if (strength < 0.06f) { Play(scrape, p, 0.25f, Random.Range(0.9f, 1.2f)); return; }
            Play(impacts[Random.Range(0, impacts.Length)], p, Mathf.Clamp01(0.25f + strength), Random.Range(0.85f, 1.15f) * (1.2f - strength * 0.4f), true, 8f);
        }

        public static void Explosion(Vector3 p) => Play(explosion, p, 1f, Random.Range(0.85f, 1.05f), true, 25f);
        public static void Gun(Vector3 p) => Play(gun, p, 0.45f, Random.Range(0.92f, 1.1f), true, 8f);
        /// <summary>Подбор бонуса слышно далеко: игрок по звуку понимает, что соперник что-то взял.</summary>
        public static void Pickup(Vector3 p) => Play(pickup, p, 0.9f, 1f, true, 30f);
        public static void Oil(Vector3 p) => Play(oil, p, 0.7f, Random.Range(0.9f, 1.1f), true, 8f);
        public static void Beep(bool final) => Play(final ? go : beep, Vector3.zero, 0.6f, 1f, false);
        public static void Horn(Vector3 p) => Play(horn, p, 0.8f, 1f, true, 20f);

        // ---------------------------------------------------------------- мотор

        /// <summary>Петля мотора: целое число периодов в секунде, поэтому шов не слышен.</summary>
        public static AudioClip EngineLoop(float baseHz, bool rough)
        {
            float f = Mathf.Round(baseHz * 0.5f) * 2f; // чётная частота: и субгармоника укладывается в петлю целиком
            float lpE = 0f;
            var r = new System.Random(rough ? 7 : 5);
            return Make("engine" + f + (rough ? "r" : ""), 1f, (t, i) =>
            {
                float ph = t * f;
                // такты цилиндров: короткие импульсы давления плюс гармоники
                float pulse = Mathf.Pow(Mathf.Max(0f, Mathf.Sin(ph * 6.2832f * 2f)), rough ? 6f : 10f);
                float body = Mathf.Sin(ph * 6.2832f) * 0.6f + Mathf.Sin(ph * 6.2832f * 2f) * 0.35f + Mathf.Sin(ph * 6.2832f * 3f) * 0.15f;
                float sub = Mathf.Sin(ph * 6.2832f * 0.5f) * 0.25f;
                lpE += ((float)(r.NextDouble() * 2 - 1) - lpE) * (rough ? 0.35f : 0.12f);
                return Mathf.Clamp((body * 0.55f + pulse * 0.5f + sub + lpE * (rough ? 0.35f : 0.12f)) * 0.7f, -1f, 1f);
            });
        }
    }
}
