# CarBang: промпты Ideogram для текстур с разрушениями

Зачем: в пакете v2 те же 8 наборов ambientCG, что и в v1, поэтому текстуры в игре не изменились. ambientCG и Poly Haven из моей среды недоступны (сеть закрыта), так что источник новых текстур: Ideogram. Из каждой картинки я сам сделаю бесшовную плитку и карты normal, roughness и AO, а у накладок вырежу фон в альфу.

## Настройки Ideogram (для всех картинок)

- Модель: последняя (3.0 или новее). Стиль: Realistic. Magic Prompt: **Off**.
- Соотношение сторон: **1:1**.
- Сгенерировать 4 варианта, выбрать 1 лучший и сделать **Upscale** (2048 px, если доступно, иначе 1024 px).
- Формат: PNG без сжатия, если есть выбор. Имя файла как в таблице ниже.
- Все файлы сложить в один zip: `CarBang_textures_v3.zip`.
- Negative prompt для группы A (поверхности):
  `text, letters, numbers, watermark, logo, signature, border, frame, perspective, horizon, sky, objects, cars, people, vignette, strong shadows, cast shadows, specular highlights, lens flare, blur, depth of field, tilt-shift, 3d render look, cartoon`
- Negative prompt для группы B (накладки на зелёном фоне):
  `text, letters, watermark, logo, border, frame, car, vehicle shape, gradient background, shadows on background, green tint on marks, perspective, vignette, blur`

Чем проверять картинку:
- в кадре нет перспективы, весь кадр занят поверхностью;
- свет ровный, без бликов и теней;
- в группе B фон ровный ярко-зелёный, на следах повреждений нет зелёного оттенка.

## A. Поверхности окружения (плитка, вид строго сверху)

Ко всем промптам группы A в конец добавить общий хвост:

`, seamless tileable texture, orthographic top-down view, perfectly flat surface filling the entire frame edge to edge, uniform diffuse overcast lighting, no shadows, no highlights, no perspective, photorealistic PBR albedo map, highly detailed, sharp`

| Файл | Где в игре | Промпт (без хвоста) |
|---|---|---|
| `env_asphalt.png` | Дороги всех карт | `old worn dark grey city asphalt road surface with fine gravel aggregate, hairline cracks, repaired tar patches and faint tire rubber marks` |
| `env_asphalt_wrecked.png` | Арены и места боёв | `badly damaged dark asphalt after a demolition derby, deep cracks, chipped potholes with loose gravel, black tire skid marks, dried oil stains, scattered tiny glass and metal fragments` |
| `env_concrete_wall.png` | Стены Колизея, отбойники | `weathered grey concrete wall surface with long horizontal scrape marks from car impacts, smeared paint transfer in red and yellow, chipped edges, rust drip stains, small spalled holes` |
| `env_concrete_floor.png` | Площадки, порт, ангары | `stained industrial concrete floor slab with expansion joint lines, oil spills, tire marks, cracks and patched areas` |
| `env_sidewalk.png` | Тротуары Даунтауна | `grey square concrete paving tiles sidewalk 40 cm tiles with thin dark joints, some tiles cracked and sunken, dirt in the joints, chewing gum spots` |
| `env_brick.png` | Кирпичные стены | `old red brick wall in running bond, soot stained, several chipped and broken bricks, crumbling mortar, bullet impact craters with exposed lighter brick` |
| `env_plaster.png` | Фасады зданий | `weathered painted plaster building wall in dull beige and grey, peeling paint revealing concrete, water streak stains, graffiti-free, small cracks` |
| `env_dirt.png` | Свалка, обочины | `compacted brown dirt ground with gravel and deep tire tracks crossing the frame, small rocks, dry cracked patches` |
| `env_sand.png` | Карьер | `coarse quarry sand and gravel ground, light ochre color, tire ruts and footprints, scattered pebbles` |
| `env_mud.png` | Лужи и грязь | `wet dark brown mud with tire ruts filled with muddy water, glossy wet patches, churned soil` |
| `env_rock.png` | Скалы карьера | `layered sedimentary rock cliff face, grey and ochre strata, fractures and broken chunks, dust in crevices` |
| `env_grass.png` | Газоны, окраины | `trampled short grass lawn with bare dirt patches and tire tracks, dry yellow areas, small weeds` |
| `env_rust_plate.png` | Ржавый металл, свалка | `heavily rusted steel plate with flaking orange rust, pitted corrosion, remains of old grey paint, rivet rows, dents` |
| `env_corrugated.png` | Контейнеры (цвет задам в игре) | `light grey painted corrugated steel shipping container wall with vertical ribs filling the frame, scratches to bare metal, rust streaks, dents from impacts` |
| `env_diamond_plate.png` | Трамплины, металлический пол | `worn steel diamond tread plate, scuffed and scratched, dirt in grooves, light rust on edges` |
| `env_planks.png` | Деревянные настилы, ящики | `weathered wooden planks laid horizontally, grey sun-bleached wood, splintered broken boards, rusty nails, dirt` |
| `env_hazard.png` | Предупреждающие полосы | `worn yellow and black diagonal hazard stripes painted on steel, heavily scratched and chipped paint, scrape marks from collisions, rust showing through` |

## B. Повреждения машины (накладки на зелёном фоне)

Накладываются поверх краски кузова по стадиям урона: 75%, 50% и 25% здоровья. Фон вырежу в прозрачность, поэтому он должен быть сплошным зелёным #00FF00.

Ко всем промптам группы B в конец добавить общий хвост:

`, isolated damage marks on a solid flat pure chroma key green background (#00FF00), the background must be a single uniform green with no texture, seamless tileable pattern, orthographic flat view, even lighting, photorealistic`

| Файл | Стадия | Промпт (без хвоста) |
|---|---|---|
| `car_damage_1.png` | Лёгкая, кузов < 100% | `light car paint damage: thin fine scratches and swirl scuffs, small paint chips, light dust and dirt speckles, covering about 15 percent of the area` |
| `car_damage_2.png` | Средняя, < 75% | `moderate car body damage: long scratches through paint to grey primer, scuffs with black rubber marks, mud splatter, small dents shown as dark creased lines, covering about 30 percent of the area` |
| `car_damage_3.png` | Тяжёлая, < 50% | `heavy car body damage: deep scrapes to bare shiny steel, torn paint edges, orange rust spots, a few bullet holes with dark centers and bent metal rims, soot smudges, covering about 45 percent of the area` |
| `car_damage_4.png` | Разбита, < 25% | `wrecked car body surface: large areas of burnt blackened paint, blistered paint bubbles, heavy flaking rust, many bullet holes, cracks and tears in metal, soot and ash, covering about 65 percent of the area` |

## C. Декали на землю (одиночные, не плитка)

Хвост для группы C: `, single isolated decal centered in frame on a solid flat pure chroma key green background (#00FF00), top-down orthographic view, soft natural edges, even lighting, photorealistic`

| Файл | Где | Промпт (без хвоста) |
|---|---|---|
| `decal_scorch.png` | След взрыва | `black scorch mark burn on the ground after an explosion, radial soot streaks, charred center, small debris` |
| `decal_crater.png` | Воронка в асфальте | `small shattered crater in asphalt, broken asphalt chunks around, radial cracks, dark gravel in the center` |
| `decal_oil.png` | Масляное пятно | `dark motor oil spill puddle with iridescent rainbow sheen at edges, irregular splash shape` |
| `decal_skid.png` | Следы шин | `two parallel black tire skid marks curving across the frame, rubber smears with tread pattern` |
| `decal_glass.png` | Осколки стекла | `scattered shattered car window glass fragments, small cubes of tempered glass, slight reflections` |

## Что я сделаю с файлами

1. Плитка (A и B): уберу швы смещением и смешиванием краёв, выровняю яркость, чтобы на ракурсе не было пятен.
2. Из цвета получу высоту, normal (OpenGL), roughness и AO. Металлические наборы (rust, corrugated, diamond, hazard) получат карту metallic по маске голого металла.
3. Накладки B и C: хромакей по зелёному, сглаживание кромки, удаление зелёного ореола.
4. Подключу вместо старых наборов. Старые ambientCG остаются запасным вариантом, если файла нет.
5. Пересниму арену, улицу и аварию в 1920×1080 и пришлю сравнение «до/после».

## Ограничения

- Карты рельефа, полученные из одной картинки, слабее сканированных PBR: глубокие трещины и кирпичи будут читаться, но без настоящей высоты.
- Ideogram не гарантирует идеальную бесшовность и ровный зелёный фон. Если вариант совсем не подходит, я напишу какой и почему, и вы перегенерируете только его.
- Альтернатива для группы A: скачать с ambientCG вручную по 1 набору 1K-JPG из категорий Asphalt, Bricks, PavingStones, Grass, Rock, Planks, CorrugatedSteel, Rust, DiamondPlate, Ground. Это настоящий PBR и лучше по качеству, чем A из Ideogram. Группы B и C всё равно лучше делать в Ideogram.

## Минимальный набор, если некогда делать всё

`env_asphalt`, `env_asphalt_wrecked`, `env_concrete_wall`, `env_sidewalk`, `env_brick`, `env_corrugated`, `env_rust_plate` и `car_damage_1..4`. Этого хватит, чтобы картинка заметно сменилась на всех картах и на машинах.
