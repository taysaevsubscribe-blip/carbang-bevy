# CarBang: трассы и эффекты, что сгенерировать и что сделаю я

Вывод: Ideogram подходит для плашек всплывашек, иконок, спрайтов частиц, плакатов, граффити и фасадов. Геометрию трасс он не даст. Уровень Carmageddon 2 по трассам достигается моделями (модульные наборы Kenney, CC0) и моей работой над картами: районы, высоты, трамплины, секреты, разрушаемый реквизит. Ассеты и названия Carmageddon не используем, только похожий жанр и подачу.

## Общие настройки Ideogram

- Стиль: Design (для плашек и иконок) или Realistic (для частиц и текстур). Magic Prompt: **Off**.
- 1:1, если не сказано иначе. Upscale до 2048 там, где доступно.
- Фон:
  - **#00FF00** для объектов с обычной прозрачностью (плашки, иконки, обломки, граффити). Вырежу фон в альфу.
  - **чистый чёрный #000000** для светящегося (огонь, искры, вспышки, взрывы, дым). Яркость станет прозрачностью.
- Negative prompt для всех: `watermark, signature, logo, brand, frame, border, mockup, photo of screen, perspective, background scene, shadow on background`
- Текст в картинках не нужен. Надписи («БАМ!», «ПОЧИНКА +25») рисую шрифтом игры поверх плашки: кириллица в Ideogram часто ломается, а шрифтом надпись чёткая и меняется без перегенерации.
- Имена файлов как в таблицах. Упаковка: `CarBang_fx_v1.zip` (группы A–C) и `CarBang_tracks_v1.zip` (группа D).

## A. Плашки всплывашек (фон #00FF00)

Хвост для группы A: `, game UI badge, bold graphic shape, thick dark outline, centered, isolated on solid flat pure chroma key green background (#00FF00), no text, no letters, high contrast, clean edges`

| Файл | Событие | Промпт (без хвоста) |
|---|---|---|
| `pop_hit_1.png` | Лёгкий удар | `small comic impact burst, yellow and orange jagged star shape, motion lines` |
| `pop_hit_2.png` | Сильный удар | `large jagged impact burst, orange to red, cracked edges, flying metal flakes` |
| `pop_hit_3.png` | Таран, сокрушительный удар | `huge explosive impact burst made of torn red metal shards and sparks, aggressive shape` |
| `pop_wreck.png` | Соперник разбит | `torn rusty metal plate badge with bolts, bent corners, scorch marks` |
| `pop_repair.png` | Починка | `green badge with crossed wrench and screwdriver, small bolts flying, glossy` |
| `pop_armor.png` | Броня | `blue steel shield badge with rivets, glossy highlight` |
| `pop_combo.png` | Комбо ×N | `rectangular yellow and black hazard tape plate, slightly tilted, worn` |
| `pop_air.png` | Полёт, переворот | `curved arrow loop badge, cyan and white, speed lines` |
| `pop_ped.png` | Очки за пешехода | `round orange target sticker badge, dashed ring` |
| `pop_bonus.png` | Бонус (общая) | `purple starburst sticker with glossy highlight and small sparkles` |
| `pop_win.png` | Победа | `wide golden banner ribbon with laurel ends and stars`, соотношение 3:1 |
| `pop_lose.png` | Поражение | `wide cracked dark steel banner with rust and smoke`, соотношение 3:1 |

## B. Старт и иконки бонусов (фон #00FF00)

Хвост тот же, что у группы A.

| Файл | Промпт (без хвоста) |
|---|---|
| `start_light_red.png` | `vertical three-lamp traffic light, only the top red lamp glowing brightly, other lamps dark, chunky cartoon style` |
| `start_light_yellow.png` | `vertical three-lamp traffic light, only the middle amber lamp glowing brightly, other lamps dark, chunky cartoon style` |
| `start_light_green.png` | `vertical three-lamp traffic light, only the bottom green lamp glowing brightly, other lamps dark, chunky cartoon style` |
| `start_go.png` | `explosive burst with a waving black and white checkered flag, sparks and speed lines` |

Иконки бонусов: по одной на тип, все в одном стиле. Хвост для иконок: `, game pickup icon, sticker style, thick black outline, flat bright colors, centered, isolated on solid flat pure chroma key green background (#00FF00), no text`

| Файл | Бонус | Что изображено |
|---|---|---|
| `ico_repair.png` | Ремонт | `wrench` |
| `ico_full_repair.png` | Полный ремонт | `wrench and gear with a plus sign shape` |
| `ico_nitro.png` | Нитро | `blue nitrous bottle with flame` |
| `ico_ammo.png` | Патроны | `ammo crate with bullets` |
| `ico_time.png` | Время | `stopwatch` |
| `ico_credits.png` | Кредиты | `stack of gold coins` |
| `ico_double_damage.png` | Двойной урон | `two crossed spiked fists` |
| `ico_shield.png` | Щит | `glowing blue force field bubble` |
| `ico_megaram.png` | Мегатаран | `huge spiked battering ram` |
| `ico_pinball.png` | Пинбол | `chrome pinball bouncing off a bumper` |
| `ico_moon.png` | Лунная гравитация | `crescent moon with a small car floating` |
| `ico_grip.png` | Суперсцепление | `tire with suction cups` |
| `ico_springs.png` | Пружины | `coil spring` |
| `ico_invisible.png` | Невидимка | `dashed outline of a car fading` |
| `ico_ped_magnet.png` | Магнит пешеходов | `red horseshoe magnet` |
| `ico_giant_peds.png` | Гиганты | `huge sneaker stepping` |
| `ico_freeze.png` | Заморозка | `snowflake crystal` |
| `ico_barrage.png` | Шквал | `three rockets in a fan` |
| `ico_mine_ring.png` | Кольцо мин | `ring of round landmines` |
| `ico_mystery.png` | Сюрприз | `wooden crate with a big question mark shape made of nails` |

## C. Спрайты эффектов

Хвост для светящихся (фон чёрный): `, isolated on pure black background (#000000), no other objects, centered, photographic VFX element, high dynamic range look`

Хвост для обломков (фон зелёный): `, isolated on solid flat pure chroma key green background (#00FF00), top-down view, no shadows`

| Файл | Фон | Промпт (без хвоста) |
|---|---|---|
| `fx_explosion_2x2.png` | чёрный | `2x2 grid of four different fireball explosion bursts, orange and yellow core, dark smoke edges, each cell separate` |
| `fx_fire_2x2.png` | чёрный | `2x2 grid of four different flame tongues rising, orange and yellow, each cell separate` |
| `fx_smoke_2x2.png` | чёрный | `2x2 grid of four different soft white smoke puffs, each cell separate` |
| `fx_dust_2x2.png` | чёрный | `2x2 grid of four different dust clouds, beige, wispy, each cell separate` |
| `fx_sparks.png` | чёрный | `burst of bright orange metal sparks flying outward with long streaks from a center point` |
| `fx_flash.png` | чёрный | `bright white impact flash star with four long thin rays and a soft glow` |
| `fx_shockwave.png` | чёрный | `thin bright distorted ring shockwave, white and light blue, seen from above` |
| `fx_muzzle_2x2.png` | чёрный | `2x2 grid of four different gun muzzle flashes seen from the side, yellow white, each cell separate` |
| `fx_repair_glow.png` | чёрный | `rising green sparkles and small plus-shaped glints, soft glow` |
| `fx_debris_2x2.png` | зелёный | `2x2 grid of four torn bent car body metal pieces with paint chips, each cell separate` |
| `fx_bolts_2x2.png` | зелёный | `2x2 grid of scattered nuts, bolts and small gears, each cell separate` |

## D. Трассы: что нужно для уровня Carmageddon 2

### D1. Модели (главное, Ideogram здесь не поможет)

Модульные наборы Kenney (CC0). Мой доступ к kenney.nl закрыт, скачать нужно вам:
- **City Kit**: коммерческий, пригородный, промышленный, дороги;
- **Racing Kit**: трибуны, ограждения, трамплины, шины;
- плюс любые наборы с мостами, ангарами, заборами.

Точные названия наборов я не проверял, на kenney.nl/assets ищите по словам «City Kit» и «Racing». Нужны архивы с GLB или GLTF.

### D2. HDRI неба

Если хотите разнообразия неба, нужно 2–3 HDRI с polyhaven.com (CC0), файлы `.hdr` 2K: ночной город, закат, пасмурно. Ideogram настоящий HDR не сделает.

### D3. Текстуры районов (Ideogram, группа A из прошлого файла: вид сверху, плитка, ровный свет)

| Файл | Промпт |
|---|---|
| `env_facade_brick_windows.png` | `old brick apartment building facade with a regular grid of windows, flat front orthographic view, seamless tileable, evening, some windows lit warm yellow` |
| `env_facade_concrete_windows.png` | `soviet style concrete panel building facade with a regular grid of windows and balconies, flat front orthographic view, seamless tileable` |
| `env_facade_shop.png` | `ground floor shop front row with roller shutters and dirty display windows, flat front orthographic view, seamless tileable horizontally, no readable signs` |
| `env_cobble.png` | `worn cobblestone street surface, top-down, seamless tileable` |
| `env_grating.png` | `rusty industrial steel floor grating, top-down, seamless tileable` |
| `env_warehouse_floor.png` | `painted warehouse concrete floor with faded yellow lines and tire marks, top-down, seamless tileable` |

### D4. Плакаты, вывески, граффити (фон #00FF00 для граффити, плакаты прямоугольные)

Все бренды выдуманные. Реальные логотипы и названия не используем.

| Файл | Промпт |
|---|---|
| `poster_1..4.png`, соотношение 2:3 | `retro advertising poster for a made-up product, bold shapes, worn and torn paper edges, no real brands` (4 разных: энергетик, автосервис, лапша, страховка) |
| `billboard_1..3.png`, соотношение 3:1 | `roadside billboard advertisement for a made-up brand, bold graphic, faded and weathered, no real brands` |
| `graffiti_1..6.png` | `street graffiti tag piece, spray paint drips, colorful, isolated on solid flat pure chroma key green background (#00FF00), no real words` |

### D5. Ключевые арты карт (настроение и цвет, не геометрия)

По 1–2 картинки 16:9 на карту, как mb1–mb3: `wide cinematic view of a [описание района] at [время суток], vehicular combat arena, wrecked cars, dramatic lighting, stylized realistic game art, no text`. Важнее всего Даунтаун и Порт.

## Что сделаю я

**Эффекты (код):**
- всплывашки: плашка, надпись шрифтом игры, отскок масштаба, подъём и затухание, сила по Δv удара;
- старт: светофор 3–2–1, «ВПЕРЁД!», облёт камеры перед стартом;
- тяжёлый удар: стоп-кадр 50–80 мс, толчок камеры, вспышка, кольцо ударной волны, обломки и болты;
- гибель соперника: замедление на 0.3 с и плашка «РАЗБИТ»;
- починка: зелёные искры и плашка с суммой;
- бонусы: иконка над машиной и в ленте;
- огонь, дым, взрывы и пыль на новых спрайтах.

**Трассы:**
- две флагманские карты переделываю первыми. Даунтаун: районы, многоуровневый паркинг с пандусами, маршрут по крышам с прыжками, тоннель, парк. Порт: штабеля контейнеров для прыжков, кран, палуба судна, склад;
- на всех картах:
  - трамплины и петли маршрутов;
  - секретные места с бонусами;
  - сбиваемый реквизит (столбы, заборы, ящики, конусы);
  - плакаты и граффити;
  - фасады с окнами.

**Честная оценка:** карта уровня Carmageddon 2: это много ручной работы. Реально по одной флагманской карте за большой заход, остальные подтягиваю следом.

## Минимальный набор, если некогда делать всё

`pop_hit_1..3`, `pop_wreck`, `pop_repair`, `start_light_*`, `start_go`, `fx_explosion_2x2`, `fx_smoke_2x2`, `fx_sparks`, `fx_flash` и наборы Kenney City Kit. Этого хватит на новые всплывашки, старт, удары и первую переделанную карту.
