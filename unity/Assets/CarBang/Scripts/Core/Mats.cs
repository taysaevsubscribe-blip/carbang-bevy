// Материалы и процедурные текстуры демо. Шейдеры URP; если меню «CarBang → Собрать демо-сцену»
// сохранило материалы в Resources/CarBangMats, берутся они (так шейдеры гарантированно попадут в сборку).
using System.Collections.Generic;
using UnityEngine;
using UnityEngine.Rendering;

namespace CarBang
{
    public static class Mats
    {
        public const string LIT = "Universal Render Pipeline/Lit";
        public const string UNLIT = "Universal Render Pipeline/Unlit";
        public const string PARTICLES = "Universal Render Pipeline/Particles/Unlit";

        static readonly Dictionary<string, Material> cache = new Dictionary<string, Material>();
        static readonly Dictionary<string, Texture2D> texCache = new Dictionary<string, Texture2D>();

        static Shader Find(string name)
        {
            var s = Shader.Find(name);
            if (s == null)
            {
                // запасной путь: шаблонный материал из Resources держит ссылку на шейдер
                var tmpl = Resources.Load<Material>("CarBangMats/" + name.Replace('/', '_'));
                if (tmpl != null) s = tmpl.shader;
            }
            if (s == null) Debug.LogError("[CarBang] Нет шейдера " + name + ": проект должен быть на URP");
            return s;
        }

        public static Material Lit(string key, Color color, float smooth = 0.4f, float metal = 0f, Texture tex = null, Vector2? tiling = null)
        {
            if (cache.TryGetValue(key, out var m)) return m;
            m = new Material(Find(LIT)) { name = key };
            m.SetColor("_BaseColor", color);
            if (tex != null)
            {
                m.SetTexture("_BaseMap", tex);
                if (tiling.HasValue) m.SetTextureScale("_BaseMap", tiling.Value);
            }
            m.SetFloat("_Smoothness", smooth);
            m.SetFloat("_Metallic", metal);
            m.enableInstancing = true;
            cache[key] = m;
            return m;
        }

        public static Material Emissive(string key, Color color, float intensity)
        {
            if (cache.TryGetValue(key, out var m)) return m;
            m = new Material(Find(LIT)) { name = key };
            m.SetColor("_BaseColor", color * 0.3f);
            m.EnableKeyword("_EMISSION");
            m.globalIlluminationFlags = MaterialGlobalIlluminationFlags.RealtimeEmissive;
            m.SetColor("_EmissionColor", color * intensity);
            cache[key] = m;
            return m;
        }

        /// <summary>Прозрачный Lit (масло, стекло). Новый экземпляр на ключ.</summary>
        public static Material LitTransparent(string key, Color color, float smooth)
        {
            if (cache.TryGetValue(key, out var m)) return m;
            m = new Material(Find(LIT)) { name = key };
            m.SetFloat("_Surface", 1f);
            m.SetFloat("_Blend", 0f);
            m.SetOverrideTag("RenderType", "Transparent");
            m.SetInt("_SrcBlend", (int)BlendMode.SrcAlpha);
            m.SetInt("_DstBlend", (int)BlendMode.OneMinusSrcAlpha);
            m.SetInt("_ZWrite", 0);
            m.EnableKeyword("_SURFACE_TYPE_TRANSPARENT");
            m.renderQueue = (int)RenderQueue.Transparent;
            m.SetColor("_BaseColor", color);
            m.SetFloat("_Smoothness", smooth);
            m.SetFloat("_Cull", 0f); // двусторонний: следы и пятна видны с любой стороны
            cache[key] = m;
            return m;
        }

        /// <summary>Материал частиц: мягкое пятно, обычное или аддитивное смешивание.</summary>
        public static Material Particle(string key, Texture tex, bool additive)
        {
            if (cache.TryGetValue(key, out var m)) return m;
            m = new Material(Find(PARTICLES)) { name = key };
            m.SetTexture("_BaseMap", tex);
            m.SetColor("_BaseColor", Color.white);
            m.SetFloat("_Surface", 1f);
            m.SetFloat("_Blend", additive ? 2f : 0f);
            m.SetOverrideTag("RenderType", "Transparent");
            m.SetInt("_SrcBlend", (int)BlendMode.SrcAlpha);
            m.SetInt("_DstBlend", additive ? (int)BlendMode.One : (int)BlendMode.OneMinusSrcAlpha);
            m.SetInt("_ZWrite", 0);
            m.EnableKeyword("_SURFACE_TYPE_TRANSPARENT");
            if (additive) m.EnableKeyword("_BLENDMODE_ADD");
            m.renderQueue = (int)RenderQueue.Transparent;
            cache[key] = m;
            return m;
        }

        public static Material Unlit(string key, Color color)
        {
            if (cache.TryGetValue(key, out var m)) return m;
            m = new Material(Find(UNLIT)) { name = key };
            m.SetColor("_BaseColor", color);
            cache[key] = m;
            return m;
        }

        // ------------------------------------------------------------------ процедурные текстуры

        static Texture2D NewTex(string key, int size, bool linear = false)
        {
            var t = new Texture2D(size, size, TextureFormat.RGBA32, true, linear)
            {
                name = key,
                wrapMode = TextureWrapMode.Repeat,
                filterMode = FilterMode.Trilinear,
                anisoLevel = 8,
            };
            return t;
        }

        /// <summary>Бесшовный шум: сумма октав синусных решёток (без внешних зависимостей).</summary>
        static readonly Dictionary<long, float[]> waveCache = new Dictionary<long, float[]>();

        /// <summary>Параметры волн (kx, ky, фаза, амплитуда) для набора октав; считаются один раз.</summary>
        static float[] Waves(int seed, int baseFreq, int octaves)
        {
            long key = ((long)seed << 32) ^ ((long)baseFreq << 8) ^ octaves;
            if (waveCache.TryGetValue(key, out var w)) return w;
            var rnd = new System.Random(seed);
            w = new float[octaves * 3 * 4];
            float amp = 1f, norm = 0f;
            int i = 0;
            for (int o = 0; o < octaves; o++)
            {
                int f = baseFreq << o;
                for (int k = 0; k < 3; k++)
                {
                    int kx = rnd.Next(-f, f + 1), ky = rnd.Next(-f, f + 1);
                    if (kx == 0 && ky == 0) kx = f;
                    w[i++] = kx; w[i++] = ky; w[i++] = (float)rnd.NextDouble() * 6.283f; w[i++] = amp;
                }
                norm += amp * 3f;
                amp *= 0.55f;
            }
            for (int j = 3; j < w.Length; j += 4) w[j] /= norm;
            waveCache[key] = w;
            return w;
        }

        static float TileNoise(float u, float v, int seed, int baseFreq, int octaves)
        {
            var w = Waves(seed, baseFreq, octaves);
            float sum = 0f;
            for (int j = 0; j < w.Length; j += 4)
                sum += w[j + 3] * Mathf.Sin((w[j] * u + w[j + 1] * v) * 6.2831853f + w[j + 2]);
            return sum;
        }

        static Texture2D Noisy(string key, int size, Color a, Color b, int seed, int freq, float grain, System.Func<float, float, Color, Color> post = null)
        {
            if (texCache.TryGetValue(key, out var t)) return t;
            t = NewTex(key, size);
            var px = new Color[size * size];
            var rnd = new System.Random(seed + 7);
            for (int y = 0; y < size; y++)
                for (int x = 0; x < size; x++)
                {
                    float u = x / (float)size, v = y / (float)size;
                    float n = TileNoise(u, v, seed, freq, 4) * 0.5f + 0.5f;
                    float g = ((float)rnd.NextDouble() - 0.5f) * grain;
                    var c = Color.Lerp(a, b, Mathf.Clamp01(n + g));
                    if (post != null) c = post(u, v, c);
                    px[y * size + x] = c;
                }
            t.SetPixels(px);
            t.Apply(true, false);
            texCache[key] = t;
            return t;
        }

        public static Texture2D Asphalt() => Noisy("asphalt", 512, new Color(0.13f, 0.13f, 0.14f), new Color(0.27f, 0.26f, 0.26f), 11, 3, 0.25f);

        /// <summary>Дорога: асфальт, белые кромки и прерывистая осевая (u поперёк, v вдоль).</summary>
        public static Texture2D Road()
        {
            if (texCache.TryGetValue("road", out var cached)) return cached;
            var baseTex = Asphalt();
            int s = 512;
            var t = NewTex("road", s);
            var src = baseTex.GetPixels();
            var px = new Color[s * s];
            for (int y = 0; y < s; y++)
                for (int x = 0; x < s; x++)
                {
                    float u = x / (float)s, v = y / (float)s;
                    var c = src[y * s + x];
                    bool edge = (u > 0.03f && u < 0.055f) || (u > 0.945f && u < 0.97f);
                    bool centre = Mathf.Abs(u - 0.5f) < 0.012f && (v % 0.5f) < 0.28f;
                    if (edge || centre) c = Color.Lerp(c, new Color(0.85f, 0.82f, 0.7f), 0.85f);
                    // тёмная полоса накатанной резины по колеям
                    float rut = Mathf.Exp(-Mathf.Pow((Mathf.Abs(u - 0.5f) - 0.2f) / 0.06f, 2f));
                    c *= 1f - rut * 0.18f;
                    c.a = 1f;
                    px[y * s + x] = c;
                }
            t.SetPixels(px);
            t.Apply(true, false);
            texCache["road"] = t;
            return t;
        }

        public static Texture2D Dirt() => Noisy("dirt", 512, new Color(0.30f, 0.22f, 0.15f), new Color(0.50f, 0.40f, 0.28f), 23, 2, 0.3f);

        public static Texture2D Concrete() => Noisy("concrete", 256, new Color(0.52f, 0.5f, 0.47f), new Color(0.72f, 0.70f, 0.66f), 31, 3, 0.18f);

        /// <summary>Жёлто-чёрные полосы для кромок барьеров и трамплина.</summary>
        public static Texture2D Hazard()
        {
            if (texCache.TryGetValue("hazard", out var cached)) return cached;
            var dirt = Noisy("hz_noise", 256, new Color(0.8f, 0.8f, 0.8f), Color.white, 41, 4, 0.2f);
            var src = dirt.GetPixels();
            int s = 256;
            var t = NewTex("hazard", s);
            var px = new Color[s * s];
            for (int y = 0; y < s; y++)
                for (int x = 0; x < s; x++)
                {
                    float d = ((x + y) / (float)s) % 0.5f;
                    var c = d < 0.25f ? new Color(0.95f, 0.72f, 0.05f) : new Color(0.08f, 0.08f, 0.08f);
                    c *= src[y * s + x].r;
                    c.a = 1f;
                    px[y * s + x] = c;
                }
            t.SetPixels(px);
            t.Apply(true, false);
            texCache["hazard"] = t;
            return t;
        }

        /// <summary>Мягкое круглое пятно для дыма, пыли и вспышек.</summary>
        public static Texture2D SoftDot(string key = "softdot", float hardness = 1.6f)
        {
            if (texCache.TryGetValue(key, out var t)) return t;
            int s = 64;
            t = new Texture2D(s, s, TextureFormat.RGBA32, true) { name = key, wrapMode = TextureWrapMode.Clamp };
            var px = new Color[s * s];
            for (int y = 0; y < s; y++)
                for (int x = 0; x < s; x++)
                {
                    float dx = (x + 0.5f) / s * 2f - 1f, dy = (y + 0.5f) / s * 2f - 1f;
                    float r = Mathf.Sqrt(dx * dx + dy * dy);
                    float a = Mathf.Pow(Mathf.Clamp01(1f - r), hardness);
                    // лёгкая неровность края, чтобы дым не выглядел кружками
                    a *= 0.85f + 0.15f * Mathf.Sin(Mathf.Atan2(dy, dx) * 5f + r * 7f);
                    px[y * s + x] = new Color(1f, 1f, 1f, Mathf.Clamp01(a));
                }
            t.SetPixels(px);
            t.Apply(true, false);
            texCache[key] = t;
            return t;
        }

        /// <summary>Пятно масла: тёмный радужный блин с рваным краем.</summary>
        public static Texture2D OilBlot()
        {
            if (texCache.TryGetValue("oil", out var t)) return t;
            int s = 128;
            t = new Texture2D(s, s, TextureFormat.RGBA32, true) { name = "oil", wrapMode = TextureWrapMode.Clamp };
            var px = new Color[s * s];
            for (int y = 0; y < s; y++)
                for (int x = 0; x < s; x++)
                {
                    float u = x / (float)s, v = y / (float)s;
                    float dx = u * 2f - 1f, dy = v * 2f - 1f;
                    float r = Mathf.Sqrt(dx * dx + dy * dy);
                    float edge = 0.75f + 0.18f * TileNoise(u, v, 5, 2, 3);
                    float a = Mathf.Clamp01((edge - r) * 12f);
                    float sheen = 0.5f + 0.5f * Mathf.Sin(r * 22f + TileNoise(u, v, 9, 3, 2) * 4f);
                    var c = Color.Lerp(new Color(0.02f, 0.02f, 0.03f), new Color(0.15f, 0.08f, 0.25f), sheen * 0.5f);
                    c.a = a * 0.92f;
                    px[y * s + x] = c;
                }
            t.SetPixels(px);
            t.Apply(true, false);
            texCache["oil"] = t;
            return t;
        }
    }
}
