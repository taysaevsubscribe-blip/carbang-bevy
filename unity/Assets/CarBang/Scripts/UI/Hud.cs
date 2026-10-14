// Интерфейс демо на uGUI, собирается кодом: панель машины, спидометр с нитро, полоса соперника,
// радар, светофор старта, всплывашки ударов, лента событий, окно селфи-камеры с репликой, пауза и итоги.
using System.Collections.Generic;
using UnityEngine;
using UnityEngine.UI;

namespace CarBang
{
    public class Hud : MonoBehaviour
    {
        public static Hud Instance;

        Font font;
        RectTransform canvasRt;
        Canvas canvas;

        // игрок
        Vehicle player, enemy;
        CarHealth playerHp, enemyHp;
        MachineGun gun;
        OilDropper oil;
        Image hpFill, armorFill, nitroFill, heatFill, enemyHpFill, enemyArmorFill;
        Text speedText, gearText, nodesText, oilText, ddText, timerText, enemyName, bigText, hintText, resultTitle, resultBody, pauseText;
        Image[] lights = new Image[3];
        GameObject lightsPanel, resultPanel, pausePanel;
        RectTransform radar;
        Image radarEnemy;
        readonly List<Image> radarPickups = new List<Image>();

        // селфи-камера
        GameObject selfiePanel;
        RawImage selfieImage;
        Text selfieName, selfieText;
        float selfieTimer;

        // всплывашки и лента
        class Pop
        {
            public RectTransform rt;
            public Text text;
            public Image plate;
            public float t, life;
            public Vector3 world;
            public bool screenFixed;
            public Vector2 screen;
        }

        readonly List<Pop> pops = new List<Pop>();
        readonly List<(Text t, float life)> feed = new List<(Text, float)>();
        RectTransform feedRoot;
        Sprite circle;

        public void Init(Vehicle p, Vehicle e)
        {
            Instance = this;
            player = p;
            enemy = e;
            playerHp = p.GetComponent<CarHealth>();
            enemyHp = e.GetComponent<CarHealth>();
            gun = p.GetComponent<MachineGun>();
            oil = p.GetComponent<OilDropper>();
            font = Resources.GetBuiltinResource<Font>("LegacyRuntime.ttf");
            circle = MakeCircle();

            canvas = gameObject.AddComponent<Canvas>();
            canvas.renderMode = RenderMode.ScreenSpaceOverlay;
            canvas.sortingOrder = 10;
            var scaler = gameObject.AddComponent<CanvasScaler>();
            scaler.uiScaleMode = CanvasScaler.ScaleMode.ScaleWithScreenSize;
            scaler.referenceResolution = new Vector2(1920, 1080);
            scaler.matchWidthOrHeight = 1f;
            gameObject.AddComponent<GraphicRaycaster>();
            canvasRt = GetComponent<RectTransform>();

            BuildPlayerPanel();
            BuildSpeedo();
            BuildEnemyBar();
            BuildRadar();
            BuildLights();
            BuildSelfie();
            feedRoot = Rect("Feed", canvasRt, new Vector2(0.5f, 0f), new Vector2(0.5f, 0f), new Vector2(0, 260), new Vector2(700, 200));
            hintText = Label(canvasRt, "Hint", "W/S газ и тормоз · A/D руль · Пробел ручник · Shift нитро · ЛКМ пулемёт · Q масло · R на колёса · V назад · C камера · Esc пауза",
                22, TextAnchor.MiddleCenter, new Vector2(0.5f, 0f), new Vector2(0, 40), new Vector2(1700, 40), new Color(1f, 1f, 1f, 0.8f));
            BuildResult();
            BuildPause();
        }

        void OnDestroy()
        {
            if (Instance == this) Instance = null;
        }

        // ------------------------------------------------------------------ построение

        static Sprite MakeCircle()
        {
            int s = 64;
            var t = new Texture2D(s, s, TextureFormat.RGBA32, false) { wrapMode = TextureWrapMode.Clamp };
            var px = new Color[s * s];
            for (int y = 0; y < s; y++)
                for (int x = 0; x < s; x++)
                {
                    float dx = x + 0.5f - s / 2f, dy = y + 0.5f - s / 2f;
                    float r = Mathf.Sqrt(dx * dx + dy * dy);
                    px[y * s + x] = new Color(1f, 1f, 1f, Mathf.Clamp01(s / 2f - r));
                }
            t.SetPixels(px);
            t.Apply();
            return Sprite.Create(t, new Rect(0, 0, s, s), new Vector2(0.5f, 0.5f));
        }

        RectTransform Rect(string name, RectTransform parent, Vector2 anchor, Vector2 pivot, Vector2 pos, Vector2 size)
        {
            var go = new GameObject(name, typeof(RectTransform));
            var rt = go.GetComponent<RectTransform>();
            rt.SetParent(parent, false);
            rt.anchorMin = rt.anchorMax = anchor;
            rt.pivot = new Vector2(0.5f, 0.5f); // позиции везде задаются центром элемента
            rt.anchoredPosition = pos;
            rt.sizeDelta = size;
            return rt;
        }

        Image Panel(RectTransform parent, string name, Vector2 anchor, Vector2 pos, Vector2 size, Color c, bool outline = true)
        {
            var rt = Rect(name, parent, anchor, anchor, pos, size);
            var img = rt.gameObject.AddComponent<Image>();
            img.color = c;
            if (outline)
            {
                var o = rt.gameObject.AddComponent<Outline>();
                o.effectColor = new Color(0f, 0f, 0f, 0.95f);
                o.effectDistance = new Vector2(3f, -3f);
            }
            return img;
        }

        Text Label(RectTransform parent, string name, string text, int size, TextAnchor align, Vector2 anchor, Vector2 pos, Vector2 box, Color color)
        {
            var rt = Rect(name, parent, anchor, anchor, pos, box);
            var t = rt.gameObject.AddComponent<Text>();
            t.font = font;
            t.text = text;
            t.fontSize = size;
            t.fontStyle = FontStyle.Bold;
            t.alignment = align;
            t.color = color;
            t.horizontalOverflow = HorizontalWrapMode.Overflow;
            t.verticalOverflow = VerticalWrapMode.Overflow;
            t.raycastTarget = false;
            var sh = rt.gameObject.AddComponent<Shadow>();
            sh.effectColor = new Color(0f, 0f, 0f, 0.8f);
            sh.effectDistance = new Vector2(2f, -2f);
            return t;
        }

        Image Bar(RectTransform parent, string name, Vector2 anchor, Vector2 pos, Vector2 size, Color fill)
        {
            Panel(parent, name + "Bg", anchor, pos, size, new Color(0.05f, 0.05f, 0.06f, 0.9f));
            var rt = Rect(name, parent, anchor, anchor, pos, size - new Vector2(6, 6));
            var img = rt.gameObject.AddComponent<Image>();
            img.color = fill;
            img.type = Image.Type.Filled;
            img.fillMethod = Image.FillMethod.Horizontal;
            img.fillOrigin = 0;
            img.sprite = Sprite.Create(Texture2D.whiteTexture, new Rect(0, 0, 4, 4), Vector2.one * 0.5f);
            return img;
        }

        void BuildPlayerPanel()
        {
            var a = new Vector2(0f, 0f);
            Panel(canvasRt, "PlayerPanel", a, new Vector2(250, 120), new Vector2(460, 190), new Color(0.11f, 0.11f, 0.13f, 0.88f));
            Label(canvasRt, "PName", player.spec.displayName.ToUpper(), 34, TextAnchor.MiddleLeft, a, new Vector2(165, 190), new Vector2(260, 40), Color.white);
            hpFill = Bar(canvasRt, "Hp", a, new Vector2(250, 155), new Vector2(420, 26), new Color(0.35f, 0.9f, 0.35f));
            armorFill = Bar(canvasRt, "Armor", a, new Vector2(250, 130), new Vector2(420, 14), new Color(0.35f, 0.65f, 1f));
            heatFill = Bar(canvasRt, "Heat", a, new Vector2(130, 100), new Vector2(180, 12), new Color(1f, 0.6f, 0.2f));
            Label(canvasRt, "HeatL", "ПУЛЕМЁТ", 16, TextAnchor.MiddleLeft, a, new Vector2(300, 100), new Vector2(140, 20), new Color(1f, 1f, 1f, 0.75f));
            oilText = Label(canvasRt, "Oil", "", 22, TextAnchor.MiddleLeft, a, new Vector2(180, 62), new Vector2(280, 30), new Color(1f, 0.9f, 0.6f));
            ddText = Label(canvasRt, "DD", "", 22, TextAnchor.MiddleLeft, a, new Vector2(380, 62), new Vector2(180, 30), new Color(1f, 0.35f, 0.25f));
            nodesText = Label(canvasRt, "Nodes", "", 18, TextAnchor.MiddleLeft, a, new Vector2(250, 240), new Vector2(460, 26), new Color(1f, 0.55f, 0.4f));
        }

        void BuildSpeedo()
        {
            var a = new Vector2(1f, 0f);
            Panel(canvasRt, "SpeedPanel", a, new Vector2(-200, 115), new Vector2(330, 180), new Color(0.11f, 0.11f, 0.13f, 0.88f));
            speedText = Label(canvasRt, "Speed", "0", 84, TextAnchor.MiddleRight, a, new Vector2(-250, 140), new Vector2(200, 90), Color.white);
            Label(canvasRt, "Kmh", "км/ч", 22, TextAnchor.MiddleLeft, a, new Vector2(-95, 118), new Vector2(90, 30), new Color(1f, 1f, 1f, 0.7f));
            gearText = Label(canvasRt, "Gear", "N", 30, TextAnchor.MiddleCenter, a, new Vector2(-95, 165), new Vector2(90, 40), new Color(1f, 0.75f, 0.2f));
            nitroFill = Bar(canvasRt, "Nitro", a, new Vector2(-200, 50), new Vector2(290, 18), new Color(0.3f, 0.7f, 1f));
            Label(canvasRt, "NitroL", "НИТРО", 16, TextAnchor.MiddleLeft, a, new Vector2(-290, 72), new Vector2(110, 20), new Color(1f, 1f, 1f, 0.75f));
        }

        void BuildEnemyBar()
        {
            var a = new Vector2(0.5f, 1f);
            Panel(canvasRt, "EnemyPanel", a, new Vector2(0, -60), new Vector2(560, 84), new Color(0.11f, 0.11f, 0.13f, 0.88f));
            enemyName = Label(canvasRt, "EnemyName", enemy.spec.displayName.ToUpper(), 26, TextAnchor.MiddleLeft, a, new Vector2(-110, -38), new Vector2(300, 30), new Color(0.6f, 0.8f, 1f));
            timerText = Label(canvasRt, "Timer", "05:00", 30, TextAnchor.MiddleRight, a, new Vector2(190, -38), new Vector2(160, 30), Color.white);
            enemyHpFill = Bar(canvasRt, "EnemyHp", a, new Vector2(0, -66), new Vector2(520, 20), new Color(0.95f, 0.3f, 0.25f));
            enemyArmorFill = Bar(canvasRt, "EnemyArmor", a, new Vector2(0, -86), new Vector2(520, 10), new Color(0.35f, 0.65f, 1f));
        }

        void BuildRadar()
        {
            var a = new Vector2(1f, 1f);
            var bg = Rect("Radar", canvasRt, a, a, new Vector2(-145, -145), new Vector2(230, 230));
            radar = bg;
            var img = bg.gameObject.AddComponent<Image>();
            img.sprite = circle;
            img.color = new Color(0.08f, 0.1f, 0.09f, 0.8f);
            var ring = Rect("Ring", bg, new Vector2(0.5f, 0.5f), new Vector2(0.5f, 0.5f), Vector2.zero, new Vector2(110, 110));
            var ri = ring.gameObject.AddComponent<Image>();
            ri.sprite = circle;
            ri.color = new Color(0.4f, 1f, 0.5f, 0.08f);
            var me = Rect("Me", bg, new Vector2(0.5f, 0.5f), new Vector2(0.5f, 0.5f), Vector2.zero, new Vector2(16, 16));
            var mi = me.gameObject.AddComponent<Image>();
            mi.sprite = circle;
            mi.color = new Color(1f, 0.85f, 0.2f);
            var en = Rect("Enemy", bg, new Vector2(0.5f, 0.5f), new Vector2(0.5f, 0.5f), Vector2.zero, new Vector2(16, 16));
            radarEnemy = en.gameObject.AddComponent<Image>();
            radarEnemy.sprite = circle;
            radarEnemy.color = new Color(1f, 0.25f, 0.2f);
            foreach (var p in Pickup.All)
            {
                var pr = Rect("P", bg, new Vector2(0.5f, 0.5f), new Vector2(0.5f, 0.5f), Vector2.zero, new Vector2(9, 9));
                var pi = pr.gameObject.AddComponent<Image>();
                pi.sprite = circle;
                radarPickups.Add(pi);
            }
        }

        void BuildLights()
        {
            var a = new Vector2(0.5f, 0.5f);
            lightsPanel = Panel(canvasRt, "Lights", a, new Vector2(0, 250), new Vector2(330, 120), new Color(0.06f, 0.06f, 0.07f, 0.95f)).gameObject;
            for (int i = 0; i < 3; i++)
            {
                var rt = Rect("L" + i, (RectTransform)lightsPanel.transform, a, a, new Vector2((i - 1) * 100, 0), new Vector2(84, 84));
                lights[i] = rt.gameObject.AddComponent<Image>();
                lights[i].sprite = circle;
                lights[i].color = new Color(0.2f, 0.05f, 0.05f);
            }
            bigText = Label(canvasRt, "Big", "", 120, TextAnchor.MiddleCenter, a, new Vector2(0, 90), new Vector2(1200, 150), Color.white);
            var o = bigText.gameObject.AddComponent<Outline>();
            o.effectColor = Color.black;
            o.effectDistance = new Vector2(5, -5);
        }

        void BuildSelfie()
        {
            var a = new Vector2(1f, 0.5f);
            selfiePanel = Panel(canvasRt, "Selfie", a, new Vector2(-230, 60), new Vector2(420, 400), new Color(0.11f, 0.11f, 0.13f, 0.92f)).gameObject;
            var rt = (RectTransform)selfiePanel.transform;
            var ir = Rect("Feed", rt, new Vector2(0.5f, 1f), new Vector2(0.5f, 1f), new Vector2(0, -161), new Vector2(392, 294));
            selfieImage = ir.gameObject.AddComponent<RawImage>();
            selfieName = Label(rt, "Name", "", 24, TextAnchor.MiddleLeft, new Vector2(0f, 0f), new Vector2(200, 72), new Vector2(380, 30), new Color(0.6f, 0.8f, 1f));
            selfieText = Label(rt, "Line", "", 24, TextAnchor.UpperLeft, new Vector2(0f, 0f), new Vector2(210, 30), new Vector2(390, 50), Color.white);
            selfieText.horizontalOverflow = HorizontalWrapMode.Wrap;
            Label(rt, "Rec", "● REC", 18, TextAnchor.MiddleLeft, new Vector2(0f, 1f), new Vector2(70, -34), new Vector2(100, 24), new Color(1f, 0.25f, 0.2f));
            selfiePanel.SetActive(false);
        }

        void BuildResult()
        {
            resultPanel = Panel(canvasRt, "Result", new Vector2(0.5f, 0.5f), Vector2.zero, new Vector2(900, 460), new Color(0.07f, 0.07f, 0.08f, 0.95f)).gameObject;
            var rt = (RectTransform)resultPanel.transform;
            resultTitle = Label(rt, "Title", "", 96, TextAnchor.MiddleCenter, new Vector2(0.5f, 1f), new Vector2(0, -90), new Vector2(880, 120), Color.white);
            resultBody = Label(rt, "Body", "", 30, TextAnchor.UpperCenter, new Vector2(0.5f, 0.5f), new Vector2(0, -60), new Vector2(860, 260), new Color(1f, 1f, 1f, 0.9f));
            resultPanel.SetActive(false);
        }

        void BuildPause()
        {
            pausePanel = Panel(canvasRt, "Pause", new Vector2(0.5f, 0.5f), Vector2.zero, new Vector2(1000, 640), new Color(0.07f, 0.07f, 0.08f, 0.95f)).gameObject;
            var rt = (RectTransform)pausePanel.transform;
            Label(rt, "PT", "ПАУЗА", 72, TextAnchor.MiddleCenter, new Vector2(0.5f, 1f), new Vector2(0, -70), new Vector2(900, 90), Color.white);
            pauseText = Label(rt, "PB", "", 26, TextAnchor.UpperLeft, new Vector2(0.5f, 0.5f), new Vector2(0, -40), new Vector2(900, 460), new Color(1f, 1f, 1f, 0.9f));
            pauseText.text =
                "Газ / тормоз: W / S (RT / LT). Задний ход: держать S на месте\n" +
                "Руль: A / D (левый стик). Ручник: Пробел (A)\n" +
                "Нитро: Shift (B или LB)\n" +
                "Пулемёт: ЛКМ, Ctrl или J (X). Перегревается: стреляйте очередями\n" +
                "Масло назад: Q или L (RB). 3 заряда, по одному за 20 с\n" +
                "Поставить на колёса: R. Взгляд назад: V. Камера: C\n\n" +
                "Совет: разозлите Бизона и уведите его в масло или к красным бочкам.\n" +
                "Бонусы: зелёный ремонт, синее нитро, красный двойной урон на горбу.\n\n" +
                "Esc: продолжить. Enter: начать заново";
            pausePanel.SetActive(false);
        }

        // ------------------------------------------------------------------ внешние вызовы

        public void SetLights(int red, bool green)
        {
            lightsPanel.SetActive(red > 0 || green);
            for (int i = 0; i < 3; i++)
            {
                if (green) lights[i].color = new Color(0.25f, 1f, 0.3f);
                else lights[i].color = i < red ? new Color(1f, 0.15f, 0.1f) : new Color(0.2f, 0.05f, 0.05f);
            }
        }

        public void HideLights() => lightsPanel.SetActive(false);

        public void Big(string text, Color c)
        {
            bigText.text = text;
            bigText.color = c;
            bigText.rectTransform.localScale = Vector3.one * 1.6f;
        }

        public void ShowLine(string who, string text, Texture feed, float seconds)
        {
            selfiePanel.SetActive(true);
            selfieImage.texture = feed;
            selfieName.text = who;
            selfieText.text = "«" + text + "»";
            selfieTimer = seconds;
        }

        public void Feed(string text, Color c)
        {
            var t = Label(feedRoot, "F", text, 26, TextAnchor.MiddleCenter, new Vector2(0.5f, 0f), Vector2.zero, new Vector2(700, 34), c);
            feed.Insert(0, (t, 3.5f));
            if (feed.Count > 4) { Destroy(feed[feed.Count - 1].t.gameObject); feed.RemoveAt(feed.Count - 1); }
        }

        /// <summary>Всплывашка: плашка с текстом у точки мира, отскок масштаба, подъём и затухание.</summary>
        public void Popup(string text, Vector3 world, Color plate, float scale = 1f)
        {
            var rt = Rect("Pop", canvasRt, Vector2.zero, new Vector2(0.5f, 0.5f), Vector2.zero, new Vector2(260, 86) * scale);
            var img = rt.gameObject.AddComponent<Image>();
            img.color = plate;
            var o = rt.gameObject.AddComponent<Outline>();
            o.effectColor = Color.black;
            o.effectDistance = new Vector2(4, -4);
            rt.localRotation = Quaternion.Euler(0, 0, Random.Range(-8f, 8f));
            var t = Label(rt, "T", text, (int)(44 * scale), TextAnchor.MiddleCenter, new Vector2(0.5f, 0.5f), Vector2.zero, rt.sizeDelta, Color.white);
            var to = t.gameObject.AddComponent<Outline>();
            to.effectColor = Color.black;
            to.effectDistance = new Vector2(3, -3);
            pops.Add(new Pop { rt = rt, text = t, plate = img, life = 1.1f, world = world });
        }

        public void ShowResult(string title, Color c, string body)
        {
            resultPanel.SetActive(true);
            resultTitle.text = title;
            resultTitle.color = c;
            resultBody.text = body;
            hintText.gameObject.SetActive(false);
        }

        public void ShowPause(bool on) => pausePanel.SetActive(on);
        public void SetTimer(float seconds) => timerText.text = $"{(int)(seconds / 60):00}:{(int)(seconds % 60):00}";
        public void HideHint() => hintText.gameObject.SetActive(false);

        // ------------------------------------------------------------------ обновление

        void Update()
        {
            if (player == null) return;
            float dt = Time.unscaledDeltaTime;
            hpFill.fillAmount = playerHp.HpFrac;
            hpFill.color = Color.Lerp(new Color(1f, 0.25f, 0.2f), new Color(0.35f, 0.9f, 0.35f), playerHp.HpFrac);
            armorFill.fillAmount = playerHp.ArmorFrac;
            nitroFill.fillAmount = player.nitro / Vehicle.NITRO_MAX;
            if (gun != null)
            {
                heatFill.fillAmount = gun.heat;
                heatFill.color = gun.overheated ? new Color(1f, 0.2f, 0.15f) : new Color(1f, 0.6f, 0.2f);
            }
            if (oil != null) oilText.text = "МАСЛО " + new string('●', oil.charges) + new string('○', OilDropper.MAX - oil.charges);
            ddText.text = playerHp.doubleDamage > 0f ? $"УРОН ×2 {playerHp.doubleDamage:0}" : "";
            var nodes = new List<string>();
            if (playerHp.RadiatorBroken) nodes.Add("радиатор");
            if (playerHp.EngineBroken) nodes.Add("двигатель");
            foreach (var w in player.wheels) if (w.punctured) { nodes.Add("колесо"); break; }
            nodesText.text = nodes.Count > 0 ? "Повреждено: " + string.Join(", ", nodes) : "";
            speedText.text = Mathf.Abs(player.SpeedKmh).ToString("0");
            gearText.text = player.Reverse ? "R" : player.Gear == 0 ? "N" : "D" + player.Gear;
            enemyHpFill.fillAmount = enemyHp.HpFrac;
            enemyArmorFill.fillAmount = enemyHp.ArmorFrac;
            enemyName.text = enemy.spec.displayName.ToUpper() + (enemyHp.doubleDamage > 0f ? "  УРОН ×2" : "");

            // радар: вверх по курсу игрока, 120 м в радиусе
            float yaw = player.transform.eulerAngles.y;
            radarEnemy.rectTransform.anchoredPosition = RadarPos(enemy.transform.position, yaw);
            radarEnemy.enabled = !enemy.wrecked;
            for (int i = 0; i < radarPickups.Count && i < Pickup.All.Count; i++)
            {
                var p = Pickup.All[i];
                radarPickups[i].rectTransform.anchoredPosition = RadarPos(p.transform.position, yaw);
                radarPickups[i].enabled = p.Available;
                radarPickups[i].color = p.kind == PickupKind.Repair ? new Color(0.3f, 1f, 0.4f) : p.kind == PickupKind.Nitro ? new Color(0.3f, 0.7f, 1f) : new Color(1f, 0.35f, 0.25f);
            }

            // большая надпись: отскок и затухание
            if (bigText.text.Length > 0)
            {
                var s = bigText.rectTransform.localScale.x;
                bigText.rectTransform.localScale = Vector3.one * Mathf.MoveTowards(s, 1f, dt * 3f);
            }

            // всплывашки
            var cam = Camera.main;
            for (int i = pops.Count - 1; i >= 0; i--)
            {
                var p = pops[i];
                p.t += dt;
                float k = p.t / p.life;
                if (k >= 1f) { Destroy(p.rt.gameObject); pops.RemoveAt(i); continue; }
                float sc = p.t < 0.12f ? Mathf.Lerp(0.4f, 1.25f, p.t / 0.12f) : Mathf.Lerp(1.25f, 1f, Mathf.Clamp01((p.t - 0.12f) / 0.15f));
                p.rt.localScale = Vector3.one * sc;
                if (cam != null)
                {
                    var sp = cam.WorldToScreenPoint(p.world + Vector3.up * (1.5f + k * 1.5f));
                    if (sp.z < 0f) { p.rt.gameObject.SetActive(false); continue; }
                    p.rt.gameObject.SetActive(true);
                    RectTransformUtility.ScreenPointToLocalPointInRectangle(canvasRt, sp, null, out var local);
                    p.rt.anchorMin = p.rt.anchorMax = new Vector2(0.5f, 0.5f);
                    p.rt.anchoredPosition = local;
                }
                float a = k > 0.7f ? 1f - (k - 0.7f) / 0.3f : 1f;
                p.text.color = new Color(1f, 1f, 1f, a);
                var pc = p.plate.color;
                pc.a = a * 0.95f;
                p.plate.color = pc;
            }

            // лента
            for (int i = feed.Count - 1; i >= 0; i--)
            {
                var (t, life) = feed[i];
                life -= dt;
                if (life <= 0f) { Destroy(t.gameObject); feed.RemoveAt(i); continue; }
                feed[i] = (t, life);
                t.rectTransform.anchoredPosition = new Vector2(0, i * 36);
                var c = t.color;
                c.a = Mathf.Clamp01(life);
                t.color = c;
            }

            if (selfieTimer > 0f)
            {
                selfieTimer -= dt;
                if (selfieTimer <= 0f) selfiePanel.SetActive(false);
            }
        }

        Vector2 RadarPos(Vector3 world, float yaw)
        {
            var d = world - player.transform.position;
            var r = Quaternion.Euler(0, -yaw, 0) * new Vector3(d.x, 0, d.z);
            var v = new Vector2(r.x, r.z) / 120f * 105f;
            if (v.magnitude > 105f) v = v.normalized * 105f;
            return v;
        }
    }
}
