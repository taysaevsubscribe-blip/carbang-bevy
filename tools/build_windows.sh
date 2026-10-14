#!/bin/bash
# Кросс-сборка CarBang под Windows из Linux: rustup stable 1.97 + исходники std той же версии
# (rust-lang/rust, тег 1.97.0, папка library с сабмодулем backtrace) + MinGW-w64.
# rust-std для windows-gnu скачать нельзя (static.rust-lang.org закрыт), поэтому std собирается из исходников.
# Использование: tools/build_windows.sh <папка_инструментов с rust197/library и wintool197/>
set -euo pipefail
TOOLS="${1:?укажи папку с rust197/library и wintool197/}"
cd "$(dirname "$0")/.."
export __CARGO_TESTS_ONLY_SRC_ROOT="$TOOLS/rust197/library"
export RUSTC_BOOTSTRAP=1
export CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER="$TOOLS/wintool197/x86_64-w64-mingw32-wrap-gcc"
cargo build --release -Zbuild-std=std,panic_abort --target x86_64-pc-windows-gnu
ls -la target/x86_64-pc-windows-gnu/release/carbang.exe
