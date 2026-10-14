// События боя для интерфейса, реплик ИИ и звука.
using System;
using UnityEngine;

namespace CarBang
{
    public enum PickupKind { Repair, Nitro, DoubleDamage }

    public static class GameEvents
    {
        /// <summary>attacker (может быть null: стена, бочка), victim, урон по кузову, точка.</summary>
        public static event Action<Vehicle, Vehicle, float, Vector3> Hit;
        public static event Action<Vehicle, Vehicle> Wrecked;
        public static event Action<Vehicle, PickupKind> Picked;
        public static event Action<Vehicle> SlippedOnOil;
        public static event Action<Vehicle, string> Node;   // пробит радиатор, двигатель, колесо

        public static void RaiseHit(Vehicle a, Vehicle v, float dmg, Vector3 p) => Hit?.Invoke(a, v, dmg, p);
        public static void RaiseWrecked(Vehicle v, Vehicle by) => Wrecked?.Invoke(v, by);
        public static void RaisePicked(Vehicle v, PickupKind k) => Picked?.Invoke(v, k);
        public static void RaiseOil(Vehicle v) => SlippedOnOil?.Invoke(v);
        public static void RaiseNode(Vehicle v, string what) => Node?.Invoke(v, what);

        /// <summary>Сброс подписок при перезапуске сцены.</summary>
        public static void Clear()
        {
            Hit = null;
            Wrecked = null;
            Picked = null;
            SlippedOnOil = null;
            Node = null;
        }
    }
}
