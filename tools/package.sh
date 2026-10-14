#!/bin/bash
# Собирает dist/: портативный zip, установщик NSIS и архив исходников.
set -euo pipefail
cd "$(dirname "$0")/.."
EXE=target/x86_64-pc-windows-gnu/release/carbang.exe
test -f "$EXE" || { echo "нет $EXE, сначала tools/build_windows.sh"; exit 1; }
VER=0.2.0
rm -rf dist && mkdir -p dist/stage/licenses
cp "$EXE" dist/stage/carbang.exe
# README в кодировке Windows (UTF-8 с BOM, CRLF), чтобы Блокнот открыл без кракозябр
printf '\xEF\xBB\xBF' > dist/stage/README.txt
sed 's/$/\r/' README.md >> dist/stage/README.txt
cat > dist/stage/LICENSE.txt <<'LIC'
MIT License

Copyright (c) 2026 CarBang

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
LIC
# CC0-ассеты: текстуры ambientCG, HDRI Poly Haven, модели Kenney (шрифты уже вшиты в exe)
mkdir -p dist/stage/assets
cp -r assets/sky assets/kenney assets/sounds assets/damage assets/skins assets/ui assets/cars assets/decals assets/fx dist/stage/assets/
# текстуры: наборы v3 (env_*) и декали ambientCG; старые наборы v1/v2 нужны только как запасные
mkdir -p dist/stage/assets/textures
cp -r assets/textures/env_* assets/textures/Leaking012A assets/textures/ManholeCover011 assets/textures/RoadLines019A dist/stage/assets/textures/
cp assets/sounds/CREDITS.txt dist/stage/licenses/CC-BY-sounds.txt
cp assets/CC0-README.txt dist/stage/licenses/CC0-assets.txt
cp assets/kenney/License-Kenney.txt dist/stage/licenses/
cp assets/fonts/OFL-RussoOne.txt dist/stage/licenses/
cp assets/fonts/LICENSE-PTFonts.txt dist/stage/licenses/
# Список сторонних библиотек с лицензиями
cargo metadata --format-version 1 --filter-platform x86_64-pc-windows-gnu 2>/dev/null | python3 -c '
import json,sys
m=json.load(sys.stdin)
ids={n["id"] for n in m["resolve"]["nodes"]}
rows=sorted((p["name"],p["version"],p.get("license") or "см. репозиторий",p.get("repository") or "") for p in m["packages"] if p["id"] in ids and p["name"]!="carbang")
out=["Сторонние библиотеки, вошедшие в carbang.exe","Все лицензии разрешительные (MIT, Apache-2.0, BSD, Zlib, ISC, CC0, Unicode).","Для двойных лицензий вида \"A OR B\" используется MIT или Apache-2.0.",""]
for n,v,l,r in rows: out.append(f"{n} {v}  [{l}]  {r}")
open("dist/stage/licenses/THIRD-PARTY.txt","w").write("\r\n".join(out)+"\r\n")
print(len(rows),"пакетов")'
(cd installer && makensis -V2 carbang.nsi)
(cd dist/stage && zip -q -r "../CarBang-${VER}-portable.zip" .)
git_files=$(ls -d Cargo.toml Cargo.lock build.rs README.md src assets tools installer/carbang.nsi installer/carbang.rc installer/carbang.ico installer/carbang_res.o installer/icon.png)
tar czf "dist/CarBang-${VER}-src.tar.gz" --transform "s,^,carbang/," $git_files
ls -la dist
