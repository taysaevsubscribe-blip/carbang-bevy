#!/bin/bash
# Linux-сборка CarBang. Архив dist/CarBang-<ver>-linux-x86_64.tar.gz с бинарником, ассетами и лицензиями.
# Бинарник собирается под glibc 2.28 (Ubuntu 20.04+, Debian 10+, Fedora 29+, Arch):
#   pip install ziglang cargo-zigbuild
#   cargo-zigbuild rustc --release --target x86_64-unknown-linux-gnu.2.28 --bin carbang -- -C link-arg=-Wl,--allow-shlib-undefined
# Затем: tools/package_linux.sh
set -euo pipefail
cd "$(dirname "$0")/.."
VER=0.2.0
BIN=target/x86_64-unknown-linux-gnu/release/carbang
test -f "$BIN" || { echo "нет $BIN, сначала cargo zigbuild (см. шапку)"; exit 1; }
NAME="CarBang-${VER}-linux-x86_64"
ST="dist/linux/$NAME"
rm -rf dist/linux && mkdir -p "$ST/licenses" "$ST/assets/textures"
cp "$BIN" "$ST/carbang"
strip "$ST/carbang" || true
cp README.md "$ST/README.md"
# лицензия кода та же, что в Windows-сборке
sed -n "/^cat > dist\/stage\/LICENSE.txt <<'LIC'$/,/^LIC$/p" tools/package.sh | sed '1d;$d' > "$ST/LICENSE.txt"
cp -r assets/sky assets/kenney assets/sounds assets/damage assets/skins assets/ui assets/cars assets/decals assets/fx "$ST/assets/"
cp -r assets/textures/env_* assets/textures/Leaking012A assets/textures/ManholeCover011 assets/textures/RoadLines019A "$ST/assets/textures/"
cp assets/sounds/CREDITS.txt "$ST/licenses/CC-BY-sounds.txt"
cp assets/CC0-README.txt "$ST/licenses/CC0-assets.txt"
cp assets/kenney/License-Kenney.txt "$ST/licenses/"
cp assets/fonts/OFL-RussoOne.txt assets/fonts/LICENSE-PTFonts.txt "$ST/licenses/"
cat > "$ST/carbang.sh" <<'SH'
#!/bin/sh
# Запуск из любой папки: ассеты лежат рядом с бинарником
cd "$(dirname "$(readlink -f "$0")")" && exec ./carbang "$@"
SH
chmod +x "$ST/carbang" "$ST/carbang.sh"
cat > "$ST/ЗАПУСК.txt" <<'TXT'
CarBang для Linux (x86_64)

Запуск: ./carbang.sh   (или ./carbang из этой папки)

Нужно:
- glibc 2.28 или новее (Ubuntu 20.04+, Debian 10+, Fedora 29+, Arch, Mint 20+);
- драйвер Vulkan: Mesa (Intel/AMD) или NVIDIA (пакеты mesa-vulkan-drivers / vulkan-radeon / nvidia);
- ALSA или PipeWire/PulseAudio (libasound2) и libudev1, есть почти везде;
- X11 или Wayland (libwayland-client0).

Ubuntu/Debian, если игра не стартует:
  sudo apt install libvulkan1 mesa-vulkan-drivers libudev1 libxkbcommon-x11-0 libwayland-client0
  sudo apt install libasound2t64   (на Ubuntu 22.04 и старше: libasound2)

Проверка без окна: ./carbang --headless --test physics
TXT
tar czf "dist/${NAME}.tar.gz" -C dist/linux "$NAME"
ls -la "dist/${NAME}.tar.gz"
