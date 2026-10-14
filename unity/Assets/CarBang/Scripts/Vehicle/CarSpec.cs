// Характеристики машины. Числа стартовые, подбираются на плейтесте.
using UnityEngine;

namespace CarBang
{
    public enum Drive { Front, Rear, All }

    [System.Serializable]
    public class CarSpec
    {
        public string id = "ferret";
        public string displayName = "Хорёк";
        /// <summary>Модель Kenney Car Kit в Resources/CarBangCars (FBX). Нет модели: машина собирается из примитивов.</summary>
        public string model = "hatchback-sports";
        /// <summary>Колёсная база, м: под неё масштабируется модель.</summary>
        public float wheelbase = 2.55f;
        public float mass = 1100f;
        /// <summary>Ускорение на старте, м/с² (без нитро).</summary>
        public float accel = 6.6f;
        /// <summary>Максимальная скорость, м/с.</summary>
        public float maxSpeed = 47f;
        public float brake = 11f;
        /// <summary>Сцепление шин (коэффициент трения).</summary>
        public float grip = 1.45f;
        public float steerMaxDeg = 33f;
        public float steerMinDeg = 8f;
        public Drive drive = Drive.All;
        public float suspensionRest = 0.34f;
        public float sag = 0.42f;
        public float damping = 0.38f;
        public float antiRoll = 0.6f;
        public float hp = 100f;
        public float armor = 50f;
        /// <summary>Множитель урона, который машина наносит тараном.</summary>
        public float ramMult = 1f;
        /// <summary>Радиус слышимости мотора на полном газу, м.</summary>
        public float noise = 120f;
        /// <summary>Базовый тон мотора, Гц (тяжёлые машины ниже).</summary>
        public float engineHz = 52f;
        public Color accent = new Color(0.9f, 0.3f, 0.1f);

        public static CarSpec Ferret()
        {
            return new CarSpec();
        }

        public static CarSpec Bison()
        {
            return new CarSpec
            {
                id = "bison",
                displayName = "Бизон",
                model = "truck",
                wheelbase = 3.1f,
                mass = 1950f,
                accel = 5.6f,
                maxSpeed = 44f,
                brake = 10f,
                grip = 1.35f,
                steerMaxDeg = 30f,
                drive = Drive.Rear,
                suspensionRest = 0.4f,
                sag = 0.45f,
                hp = 100f,
                armor = 50f,
                ramMult = 1.15f,
                noise = 150f,
                engineHz = 38f,
                accent = new Color(0.15f, 0.55f, 0.95f),
            };
        }
    }

    public struct VehicleInput
    {
        public float Throttle;
        public float Brake;
        public float Steer;
        public bool Handbrake;
        public bool Nitro;
    }
}
