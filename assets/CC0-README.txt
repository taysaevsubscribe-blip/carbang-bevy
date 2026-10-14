CarBang — бесплатные ассеты (CC0) для Bevy-билда, пакет v1
===========================================================

Лицензии: всё CC0 (public domain) — можно использовать в коммерческой игре,
атрибуция не требуется (но приятна).

Источники:
  - Текстуры PBR: ambientCG.com (CC0)
  - Модели/пропсы: kenney.nl (CC0)
  - Небо (HDRI): polyhaven.com (CC0)

Состав:

1) textures_pbr/  (1K JPG, бесшовные, PBR)
   Road012A       — асфальт дороги с трещинами     (Color, AO, NormalGL, Roughness)
   Concrete034    — бетон (стены, отбойники)       (Color, NormalGL, Roughness)
   Ground037      — земля/грязь (обочины)          (Color, AO, NormalGL, Roughness)
   Metal032       — ржавый металл (корпуса, обломки) (Color, NormalGL, Roughness)
   Leaking012A    — потёки на стенах (используется только Roughness)
   RoadLines019A  — дорожная разметка (декаль, есть Opacity)
   ManholeCover011— люк (декаль/пропс, есть Opacity)

   Подключение в Bevy: Color -> base_color_texture, NormalGL -> normal_map_texture,
   Roughness -> metallic_roughness (шероховатость в G-канале, металличность в B),
   AO -> occlusion_texture. Для тайлинга повторитель: Repeat::Repeat + UV scale 4-16.

2) sky/  (HDRI 1K, .hdr)
   dikhololo_night.hdr      — ночное небо
   kloppenheim_02_puresky.hdr — дневное/сумеречное небо

3) kenney/  (исходные zip-киты, внутри .glb/.obj/.fbx + PNG)
   kenney_car-kit.zip        — low-poly машины и детали (50 моделей)
   kenney_city-kit-roads.zip — дороги, развязки, бордюры (95 моделей)
   kenney_nature-kit.zip     — деревья, камни, кусты (329 моделей)
   Бери .glb — Bevy загружает glTF Scene напрямую (SceneRoot).

Как отдать Клоду: положить эту папку/архив в рабочую папку проекта (Bevy assets/),
он подключит текстуры вместо процедурных. Формат файлов — обычные JPG/PNG/HDR/GLB.

В сборку включены только используемые карты, перепакованные в JPEG q86 (1K).

Модели машин (assets/cars): Kenney Car Kit, CC0 (https://kenney.nl). Переведены в формат игры
скриптом tools/cars/convert.py: масштаб, разделение на краску/отделку/стекло, отрывные бамперы.

Текстуры v3 (assets/textures/env_*, assets/damage, assets/decals): созданы для CarBang в Ideogram
по промптам проекта, карты normal/roughness/AO построены по цвету; обработка tools/textures/prepare_v3.py.

Материалы машин (assets/cars/tex): сгенерированы tools/cars/car_textures.py. Царапины взяты из
car_damage_1 (пакет v3, Ideogram, сделан для CarBang), потёки из Leaking012A (ambientCG, CC0),
рифлёная сталь бамперов из env_diamond_plate (пакет v3), остальное процедурно. Затенение и грязь
запекаются в модели tools/cars/bake.py.
