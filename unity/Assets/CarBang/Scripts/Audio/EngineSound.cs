// Мотор машины: два слоя (ровный и рычащий под нагрузкой), тон по оборотам, громкость по газу.
// За стеной звук глухой (фильтр низких частот): по звуку можно понять, где соперник и закрыт ли он.
using UnityEngine;

namespace CarBang
{
    public class EngineSound : MonoBehaviour
    {
        Vehicle veh;
        AudioSource smooth, rough;
        AudioLowPassFilter lowpass;
        float occl;
        readonly RaycastHit[] hits = new RaycastHit[6];

        public void Init(Vehicle v)
        {
            veh = v;
            var holder = new GameObject("Engine");
            holder.transform.SetParent(transform, false);
            holder.transform.localPosition = new Vector3(0f, 0.6f, 1.2f);
            smooth = Src(holder, Sfx.EngineLoop(v.spec.engineHz, false));
            rough = Src(holder, Sfx.EngineLoop(v.spec.engineHz * 1.5f, true));
            lowpass = holder.AddComponent<AudioLowPassFilter>();
            lowpass.cutoffFrequency = 22000f;
        }

        AudioSource Src(GameObject g, AudioClip c)
        {
            var s = g.AddComponent<AudioSource>();
            s.clip = c;
            s.loop = true;
            s.spatialBlend = 1f;
            s.rolloffMode = AudioRolloffMode.Logarithmic;
            s.minDistance = 4f;
            s.maxDistance = Mathf.Max(60f, veh.spec.noise * 2f);
            s.dopplerLevel = 0.6f;
            s.volume = 0f;
            s.Play();
            return s;
        }

        void Update()
        {
            if (veh == null) return;
            float rpm = veh.Rpm01;
            float load = veh.wrecked ? 0f : Mathf.Max(veh.Load01, veh.NitroActive ? 1f : 0f);
            float pitch = Mathf.Lerp(0.55f, 2.0f, rpm);
            smooth.pitch = pitch;
            rough.pitch = pitch * 0.98f;
            float loud = Mathf.Clamp01(veh.spec.noise / 150f);
            float target = veh.wrecked ? 0f : Mathf.Lerp(0.3f, 0.75f, rpm) * loud;
            // свой мотор игрок слышит потише, чтобы различать соперников
            if (veh.isPlayer) target *= 0.85f;
            smooth.volume = Mathf.MoveTowards(smooth.volume, target * (1f - load * 0.5f), Time.deltaTime * 2f);
            rough.volume = Mathf.MoveTowards(rough.volume, target * load * 0.9f, Time.deltaTime * 3f);

            // перекрытие: луч от слушателя до мотора упирается в неподвижную преграду
            var cam = Camera.main;
            if (cam != null && !veh.isPlayer)
            {
                Vector3 a = cam.transform.position, b = smooth.transform.position;
                bool blocked = false;
                int n = Physics.RaycastNonAlloc(a, (b - a).normalized, hits, Vector3.Distance(a, b), ~0, QueryTriggerInteraction.Ignore);
                for (int i = 0; i < n; i++) if (hits[i].rigidbody == null) { blocked = true; break; }
                occl = Mathf.MoveTowards(occl, blocked ? 1f : 0f, Time.deltaTime * 3f);
                lowpass.cutoffFrequency = Mathf.Lerp(22000f, 900f, occl);
            }
        }
    }
}
