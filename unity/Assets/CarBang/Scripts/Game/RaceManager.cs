// Ход заезда: светофор 3-2-1, бой до 5 минут, замедление на гибели, итоги, пауза, перезапуск.
// Плюс водитель-игрок: ввод в машину, пулемёт, масло, эвакуация на колёса.
using UnityEngine;
using UnityEngine.SceneManagement;

namespace CarBang
{
    public class PlayerDriver : MonoBehaviour
    {
        Vehicle veh;
        MachineGun gun;
        OilDropper oil;
        float resetCooldown;

        public void Init(Vehicle v)
        {
            veh = v;
            gun = GetComponent<MachineGun>();
            oil = GetComponent<OilDropper>();
        }

        void Update()
        {
            if (veh == null || Time.timeScale == 0f) return;
            veh.input = new VehicleInput
            {
                Throttle = GameInput.Throttle(),
                Brake = GameInput.Brake(),
                Steer = GameInput.Steer(),
                Handbrake = GameInput.Held(Btn.Handbrake),
                Nitro = GameInput.Held(Btn.Nitro),
            };
            if (gun != null) gun.trigger = GameInput.Held(Btn.Fire);
            if (oil != null && GameInput.Pressed(Btn.Oil)) oil.request = true;
            if (GameInput.Pressed(Btn.Horn)) Sfx.Horn(transform.position);
            resetCooldown -= Time.deltaTime;
            if (GameInput.Pressed(Btn.Reset) && resetCooldown <= 0f && !veh.wrecked && !veh.locked && TrackInfo.Current != null)
            {
                resetCooldown = 3f;
                TrackInfo.Current.ResetPose(transform.position, transform.forward, out var p, out var r);
                veh.ResetTo(p, r);
                var h = GetComponent<CarHealth>();
                if (h != null) h.invulnerable = 1.5f;
            }
        }
    }

    public class RaceManager : MonoBehaviour
    {
        public Vehicle player, enemy;
        public float timeLimit = 300f;
        enum Phase { Countdown, Fight, Ending, Done }
        Phase phase = Phase.Countdown;
        float t;
        float clock;
        float slowmo;
        bool paused;
        int lastCount = -1;
        string result;
        float hitsOnEnemy, hitsOnPlayer;

        public void Init(Vehicle p, Vehicle e)
        {
            player = p;
            enemy = e;
            clock = timeLimit;
            GameEvents.Hit += OnHit;
            GameEvents.Wrecked += OnWrecked;
            GameEvents.Picked += OnPicked;
            GameEvents.Node += OnNode;
            GameEvents.SlippedOnOil += OnOil;
        }

        void OnDestroy()
        {
            GameEvents.Clear();
            Time.timeScale = 1f;
        }

        void OnHit(Vehicle a, Vehicle v, float dmg, Vector3 p)
        {
            var hud = Hud.Instance;
            if (hud == null) return;
            if (v == enemy && a == player)
            {
                hitsOnEnemy += dmg;
                if (dmg >= 3f)
                {
                    string w = dmg >= 25f ? "В ХЛАМ!" : dmg >= 12f ? "ХРЯСЬ!" : "БАМ!";
                    var c = dmg >= 25f ? new Color(0.85f, 0.1f, 0.08f) : dmg >= 12f ? new Color(0.95f, 0.4f, 0.05f) : new Color(0.95f, 0.75f, 0.1f);
                    hud.Popup(w + " " + Mathf.RoundToInt(dmg), p, c, dmg >= 25f ? 1.3f : 1f);
                }
            }
            else if (v == player && dmg >= 6f)
            {
                hitsOnPlayer += dmg;
                hud.Popup("-" + Mathf.RoundToInt(dmg), p, new Color(0.25f, 0.25f, 0.3f), 0.8f);
            }
        }

        void OnWrecked(Vehicle v, Vehicle by)
        {
            if (phase != Phase.Fight) return;
            slowmo = 1.2f;
            Time.timeScale = 0.3f;
            var hud = Hud.Instance;
            if (v == enemy)
            {
                result = "win";
                hud?.Popup("РАЗБИТ!", v.transform.position, new Color(0.45f, 0.3f, 0.2f), 1.5f);
            }
            else if (v == player) result = "lose";
            phase = Phase.Ending;
            t = 0f;
        }

        void OnPicked(Vehicle v, PickupKind k)
        {
            var hud = Hud.Instance;
            if (hud == null) return;
            string name = k == PickupKind.Repair ? "РЕМОНТ +35" : k == PickupKind.Nitro ? "НИТРО" : "УРОН ×2";
            var c = k == PickupKind.Repair ? new Color(0.2f, 0.75f, 0.3f) : k == PickupKind.Nitro ? new Color(0.2f, 0.5f, 0.95f) : new Color(0.9f, 0.2f, 0.15f);
            if (v == player) hud.Popup(name, v.transform.position, c, 1.1f);
            else hud.Feed(v.spec.displayName + " взял: " + name.ToLower(), new Color(1f, 0.8f, 0.5f));
        }

        void OnNode(Vehicle v, string what)
        {
            if (Hud.Instance == null) return;
            if (v == player) Hud.Instance.Feed("У вас: " + what.ToLower(), new Color(1f, 0.5f, 0.4f));
            else Hud.Instance.Feed(v.spec.displayName + ": " + what.ToLower(), new Color(0.7f, 0.9f, 1f));
        }

        void OnOil(Vehicle v)
        {
            if (v == enemy && Hud.Instance != null) Hud.Instance.Popup("ЗАНОС!", v.transform.position, new Color(0.2f, 0.15f, 0.3f), 1f);
        }

        void Update()
        {
            var hud = Hud.Instance;
            float dt = Time.unscaledDeltaTime;

            if (GameInput.Pressed(Btn.Pause) && phase != Phase.Done)
            {
                paused = !paused;
                Time.timeScale = paused ? 0f : 1f;
                AudioListener.pause = paused;
                hud?.ShowPause(paused);
            }
            if ((paused || phase == Phase.Done) && GameInput.Pressed(Btn.Restart))
            {
                Time.timeScale = 1f;
                AudioListener.pause = false;
                SceneManager.LoadScene(SceneManager.GetActiveScene().buildIndex);
                return;
            }
            if (paused) return;

            if (slowmo > 0f)
            {
                slowmo -= dt;
                if (slowmo <= 0f) Time.timeScale = 1f;
            }

            switch (phase)
            {
                case Phase.Countdown:
                {
                    t += dt;
                    // 0,5 с пауза, затем три красных по секунде и зелёный
                    int count = t < 0.5f ? 0 : Mathf.Min(4, 1 + (int)(t - 0.5f));
                    if (count != lastCount)
                    {
                        lastCount = count;
                        if (count >= 1 && count <= 3)
                        {
                            hud?.SetLights(count, false);
                            hud?.Big((4 - count).ToString(), new Color(1f, 0.85f, 0.3f));
                            Sfx.Beep(false);
                            if (count == 1)
                            {
                                var voice = enemy.GetComponent<AIVoice>();
                                if (voice != null) voice.Say(Line.Start, true);
                            }
                        }
                        else if (count == 4)
                        {
                            hud?.SetLights(0, true);
                            hud?.Big("ВПЕРЁД!", new Color(0.3f, 1f, 0.35f));
                            Sfx.Beep(true);
                            player.locked = false;
                            enemy.locked = false;
                            player.GetComponent<CarHealth>().invulnerable = 1.5f;
                            enemy.GetComponent<CarHealth>().invulnerable = 1.5f;
                            phase = Phase.Fight;
                            t = 0f;
                        }
                    }
                    break;
                }
                case Phase.Fight:
                    t += dt;
                    if (t > 1.2f && t < 1.3f) { hud?.HideLights(); hud?.Big("", Color.white); }
                    if (t > 12f) hud?.HideHint();
                    clock -= Time.deltaTime;
                    hud?.SetTimer(Mathf.Max(0f, clock));
                    if (clock <= 0f)
                    {
                        result = "time";
                        phase = Phase.Ending;
                        t = 0f;
                    }
                    break;
                case Phase.Ending:
                    t += dt;
                    if (t > 2.6f)
                    {
                        phase = Phase.Done;
                        Time.timeScale = 1f;
                        ShowResult();
                    }
                    break;
            }
        }

        void ShowResult()
        {
            var hud = Hud.Instance;
            if (hud == null) return;
            var ph = player.GetComponent<CarHealth>();
            var eh = enemy.GetComponent<CarHealth>();
            float used = timeLimit - Mathf.Max(0f, clock);
            string stats = $"Время боя: {(int)(used / 60)}:{(int)(used % 60):00}\n" +
                           $"Урон по сопернику: {Mathf.RoundToInt(hitsOnEnemy)}\n" +
                           $"Получено урона: {Mathf.RoundToInt(hitsOnPlayer)}\n" +
                           $"Ваш кузов: {Mathf.RoundToInt(ph.hp)} · кузов соперника: {Mathf.RoundToInt(eh.hp)}\n\n" +
                           "Enter: ещё раз";
            switch (result)
            {
                case "win": hud.ShowResult("ПОБЕДА", new Color(1f, 0.8f, 0.2f), enemy.spec.displayName + " разбит.\n" + stats); break;
                case "lose": hud.ShowResult("РАЗБИТ", new Color(0.9f, 0.25f, 0.2f), enemy.spec.displayName + " оказался крепче.\n" + stats); break;
                default:
                    bool win = hitsOnEnemy > hitsOnPlayer;
                    hud.ShowResult(win ? "ПОБЕДА ПО ОЧКАМ" : "ВРЕМЯ ВЫШЛО", win ? new Color(1f, 0.8f, 0.2f) : new Color(0.8f, 0.8f, 0.85f), stats);
                    break;
            }
        }
    }
}
