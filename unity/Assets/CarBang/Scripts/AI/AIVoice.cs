// Реплики соперника с окном селфи-камеры: маленькая камера смотрит на его машину, пока он говорит.
// Озвучки пока нет: текст на экране и короткий звук рации. Одна реплика не чаще раза в 7 с, без повторов.
using System.Collections.Generic;
using UnityEngine;

namespace CarBang
{
    public enum Line { Start, Taunt, Hurt, Revenge, Oil, PickRepair, PickNitro, PickDamage, LowHp, Wrecked, Victory, Explosion, Ambush }

    public class AIVoice : MonoBehaviour
    {
        public string driverName = "Гоша Бизон";
        Camera selfie;
        public RenderTexture Feed { get; private set; }
        float cooldown;
        float showTimer;
        readonly HashSet<string> used = new HashSet<string>();

        static readonly Dictionary<Line, string[]> Lines = new Dictionary<Line, string[]>
        {
            { Line.Start, new[] { "Бизон на старте. Готовь запчасти!", "Ну что, малыш, покатаемся?" } },
            { Line.Taunt, new[] { "Посторонись, малыш!", "Это был бампер. Мой бампер.", "Ещё разок? С удовольствием!", "Вмятинка на память!" } },
            { Line.Hurt, new[] { "Ах ты ж... Запомню!", "Ты мне фару разбил!", "Всё, теперь это личное.", "Эй! Я её только покрасил!" } },
            { Line.Revenge, new[] { "Должок вернул!", "Я же говорил, найду.", "Сдачи не надо!" } },
            { Line.Oil, new[] { "Масло?! Кто разлил масло?!", "Скользко... очень скользко!", "Ты за это ответишь, химик!" } },
            { Line.PickRepair, new[] { "Ремонт! Как новенький. Почти.", "Подлатался. Продолжаем." } },
            { Line.PickNitro, new[] { "Нитро! Ну держись.", "Полный бак веселья!" } },
            { Line.PickDamage, new[] { "Двойной урон! Кто не спрятался...", "Теперь бью вдвое больнее." } },
            { Line.LowHp, new[] { "Дымлю, но еду!", "Мотор, не бросай меня...", "Так, где тут ремонт?" } },
            { Line.Wrecked, new[] { "Это была... прекрасная поездка...", "Скажите маме, что я ехал красиво." } },
            { Line.Victory, new[] { "Следующий!", "Металлолом сдан.", "Бизон снова в деле!" } },
            { Line.Explosion, new[] { "Кто тут бочки расставил?!", "Уши заложило!" } },
            { Line.Ambush, new[] { "Тссс... Сиди тихо, Гоша.", "Подожду тут. Он сам приедет." } },
        };

        public void Init(string name)
        {
            driverName = name;
            Feed = new RenderTexture(384, 288, 16) { name = "SelfieCam" };
            var go = new GameObject("SelfieCam");
            go.transform.SetParent(transform, false);
            // камера на капоте смотрит в лобовое стекло, как экшн-камера
            var col = GetComponent<BoxCollider>();
            float front = col != null ? col.center.z + col.size.z * 0.5f : 2f;
            float top = col != null ? col.center.y + col.size.y * 0.5f : 1.4f;
            go.transform.localPosition = new Vector3(0.5f, top + 0.35f, front + 1.6f);
            go.transform.LookAt(transform.TransformPoint(new Vector3(0f, top - 0.1f, front - 1.2f)));
            selfie = go.AddComponent<Camera>();
            selfie.targetTexture = Feed;
            selfie.fieldOfView = 55f;
            selfie.nearClipPlane = 0.1f;
            selfie.farClipPlane = 120f;
            selfie.enabled = false;
        }

        void Update()
        {
            if (cooldown > 0f) cooldown -= Time.unscaledDeltaTime;
            if (showTimer > 0f)
            {
                showTimer -= Time.unscaledDeltaTime;
                // лёгкая тряска селфи-камеры
                selfie.transform.localRotation *= Quaternion.Euler(Random.Range(-0.3f, 0.3f), Random.Range(-0.3f, 0.3f), 0f);
                if (showTimer <= 0f) selfie.enabled = false;
            }
        }

        public void Say(Line line, bool force = false)
        {
            if (!force && cooldown > 0f) return;
            if (!Lines.TryGetValue(line, out var opts)) return;
            var fresh = new List<string>();
            foreach (var o in opts) if (!used.Contains(o)) fresh.Add(o);
            if (fresh.Count == 0) { foreach (var o in opts) used.Remove(o); fresh.AddRange(opts); }
            string text = fresh[Random.Range(0, fresh.Count)];
            used.Add(text);
            cooldown = 7f;
            showTimer = 3.2f;
            selfie.enabled = true;
            if (Hud.Instance != null) Hud.Instance.ShowLine(driverName, text, Feed, 3.2f);
            Sfx.Beep(false);
        }
    }
}
