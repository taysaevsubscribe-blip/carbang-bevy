// Правила урона, перенесённые из прототипа на Bevy (src/balance.rs, проверены автотестами там).
// Чистая логика без UnityEngine: её можно проверять обычными тестами.
using System;

namespace CarBang
{
    public struct HitResult
    {
        public float Hp;
        public float Armor;
        public float HpLoss;
        public bool Wrecked;
    }

    public static class DamageRules
    {
        /// <summary>Урон за удар: 7,2 × max(0, |Δv| − 4 м/с).</summary>
        public const float K_DV = 7.2f;
        public const float DV_FREE = 4f;
        /// <summary>Доля урона, которую берёт броня, пока не кончится.</summary>
        public const float ARMOR_SHARE = 0.5f;
        /// <summary>Машина с кузовом от этого значения переживает любой одиночный удар (остаётся на 1).</summary>
        public const float ONE_SHOT_GUARD_HP = 25f;
        /// <summary>Мгновенная гибель: кузов ниже 25 и удар быстрее 120 км/ч.</summary>
        public const float INSTANT_DEATH_SPEED = 120f / 3.6f;
        /// <summary>Множитель урона оружия: таран важнее стрельбы.</summary>
        public const float WEAPON_DAMAGE_MULT = 0.15f;

        public static float ImpactDamage(float deltaV)
        {
            return K_DV * Math.Max(0f, Math.Abs(deltaV) - DV_FREE);
        }

        public static HitResult ApplyHit(float hp, float armor, float damage, float impactSpeed)
        {
            float absorbed = Math.Min(damage * ARMOR_SHARE, Math.Max(armor, 0f));
            float armorLeft = armor - absorbed;
            float toHp = Math.Max(damage - absorbed, 0f);
            if (hp < ONE_SHOT_GUARD_HP && impactSpeed > INSTANT_DEATH_SPEED)
                return new HitResult { Hp = 0f, Armor = armorLeft, HpLoss = hp, Wrecked = true };
            float newHp = hp - toHp;
            if (hp >= ONE_SHOT_GUARD_HP && newHp <= 0f) newHp = 1f;
            return new HitResult { Hp = Math.Max(newHp, 0f), Armor = armorLeft, HpLoss = hp - Math.Max(newHp, 0f), Wrecked = newHp <= 0f };
        }

        /// <summary>Стадия износа по доле кузова: больше 75% — 1, 50% — 2, 25% — 3, иначе 4.</summary>
        public static int DamageStage(float hpFrac)
        {
            if (hpFrac > 0.75f) return 1;
            if (hpFrac > 0.5f) return 2;
            if (hpFrac > 0.25f) return 3;
            return 4;
        }
    }
}
