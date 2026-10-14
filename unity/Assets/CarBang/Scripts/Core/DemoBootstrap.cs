// Точка входа демо: строит всю сцену кодом при запуске. В сцене нужен только объект с этим компонентом
// (меню CarBang → Собрать демо-сцену делает его сам).
using UnityEngine;
using UnityEngine.Rendering;
using UnityEngine.Rendering.Universal;

namespace CarBang
{
    public class DemoBootstrap : MonoBehaviour
    {
        [Tooltip("Ярость Бизона на старте и его характер можно менять прямо тут")]
        public float enemySkill = 0.75f;
        public float enemyAggression = 0.8f;
        public float enemyCunning = 0.6f;

        void Awake()
        {
            Time.fixedDeltaTime = 1f / 120f;
            Time.timeScale = 1f;
            Application.targetFrameRate = -1;
            GameEvents.Clear();

            Fx.Init();
            Sfx.Init();
            var sun = Environment();
            var cam = SetupCamera();
            var track = Arena.Build();

            // старт: восточная прямая, обе машины носом против часовой стрелки
            var start = new Vector3(Arena.A, 0f, -6f);
            var rot = Quaternion.LookRotation(Vector3.forward, Vector3.up);
            var player = CarFactory.Build(CarSpec.Ferret(), start + new Vector3(-3.6f, 1.0f, 0f), rot, true);
            var enemy = CarFactory.Build(CarSpec.Bison(), start + new Vector3(3.6f, 1.2f, 0f), rot, false);
            Arena.Cars.Add(player);
            Arena.Cars.Add(enemy);

            Arm(player);
            Arm(enemy);
            player.gameObject.AddComponent<PlayerDriver>().Init(player);
            var voice = enemy.gameObject.AddComponent<AIVoice>();
            voice.Init("Гоша Бизон");
            var ai = enemy.gameObject.AddComponent<AIDriver>();
            ai.skill = enemySkill;
            ai.aggression = enemyAggression;
            ai.cunning = enemyCunning;
            ai.Init(enemy, player, track);
            // соперник стреляет чуть хуже игрока
            enemy.GetComponent<MachineGun>().damagePerHit = 0.8f;

            cam.GetComponent<ChaseCamera>().target = player;
            cam.transform.position = player.transform.position - player.transform.forward * 8f + Vector3.up * 3f;

            var hudGo = new GameObject("HUD");
            hudGo.AddComponent<Hud>().Init(player, enemy);
            gameObject.AddComponent<RaceManager>().Init(player, enemy);
        }

        static void Arm(Vehicle v)
        {
            var col = v.GetComponent<BoxCollider>();
            var top = col.center + new Vector3(0f, col.size.y * 0.5f + 0.18f, col.size.z * 0.12f);
            v.gameObject.AddComponent<MachineGun>().Init(v, top + Vector3.forward * 0.5f);
            v.gameObject.AddComponent<OilDropper>().Init(v);
        }

        /// <summary>Закат: низкое тёплое солнце, процедурное небо, дымка, пост-обработка с яркой палитрой.</summary>
        static Light Environment()
        {
            foreach (var l in FindObjectsByType<Light>(FindObjectsSortMode.None))
                if (l.type == LightType.Directional) Destroy(l.gameObject);

            var sunGo = new GameObject("Sun");
            var sun = sunGo.AddComponent<Light>();
            sun.type = LightType.Directional;
            sun.color = new Color(1f, 0.72f, 0.48f);
            sun.intensity = 2.1f;
            sun.shadows = LightShadows.Soft;
            sun.shadowStrength = 0.85f;
            sun.shadowBias = 0.05f;
            sun.shadowNormalBias = 0.5f;
            sunGo.transform.rotation = Quaternion.Euler(16f, -35f, 0f);

            var sky = Resources.Load<Material>("CarBangMats/Sky");
            if (sky == null)
            {
                var sh = Shader.Find("Skybox/Procedural");
                if (sh != null) sky = new Material(sh);
            }
            if (sky != null)
            {
                sky.SetFloat("_SunSize", 0.06f);
                sky.SetFloat("_SunSizeConvergence", 4f);
                sky.SetFloat("_AtmosphereThickness", 1.35f);
                sky.SetColor("_SkyTint", new Color(0.55f, 0.5f, 0.6f));
                sky.SetColor("_GroundColor", new Color(0.32f, 0.26f, 0.22f));
                sky.SetFloat("_Exposure", 1.15f);
                RenderSettings.skybox = sky;
            }
            RenderSettings.sun = sun;
            RenderSettings.ambientMode = AmbientMode.Trilight;
            RenderSettings.ambientSkyColor = new Color(0.55f, 0.52f, 0.62f);
            RenderSettings.ambientEquatorColor = new Color(0.52f, 0.4f, 0.32f);
            RenderSettings.ambientGroundColor = new Color(0.2f, 0.16f, 0.13f);
            RenderSettings.fog = true;
            RenderSettings.fogMode = FogMode.Exponential;
            RenderSettings.fogDensity = 0.0032f;
            RenderSettings.fogColor = new Color(0.78f, 0.6f, 0.5f);
            RenderSettings.reflectionIntensity = 0.8f;

            // пост-обработка: яркая и сочная картинка без «грязи»
            var volGo = new GameObject("PostFX");
            var vol = volGo.AddComponent<Volume>();
            vol.isGlobal = true;
            var prof = ScriptableObject.CreateInstance<VolumeProfile>();
            var bloom = prof.Add<Bloom>(true);
            bloom.threshold.Override(1.05f);
            bloom.intensity.Override(0.55f);
            bloom.scatter.Override(0.65f);
            var tone = prof.Add<Tonemapping>(true);
            tone.mode.Override(TonemappingMode.ACES);
            var ca = prof.Add<ColorAdjustments>(true);
            ca.postExposure.Override(0.25f);
            ca.contrast.Override(12f);
            ca.saturation.Override(18f);
            var vig = prof.Add<Vignette>(true);
            vig.intensity.Override(0.22f);
            vig.smoothness.Override(0.45f);
            var wb = prof.Add<WhiteBalance>(true);
            wb.temperature.Override(8f);
            vol.sharedProfile = prof;
            return sun;
        }

        static Camera SetupCamera()
        {
            var cam = Camera.main;
            if (cam == null)
            {
                var go = new GameObject("Main Camera");
                go.tag = "MainCamera";
                cam = go.AddComponent<Camera>();
                go.AddComponent<AudioListener>();
            }
            cam.nearClipPlane = 0.1f;
            cam.farClipPlane = 900f;
            cam.fieldOfView = 62f;
            cam.allowHDR = true;
            var data = cam.GetUniversalAdditionalCameraData();
            data.renderPostProcessing = true;
            data.antialiasing = AntialiasingMode.SubpixelMorphologicalAntiAliasing;
            data.antialiasingQuality = AntialiasingQuality.High;
            if (cam.GetComponent<ChaseCamera>() == null) cam.gameObject.AddComponent<ChaseCamera>();
            return cam;
        }
    }
}
