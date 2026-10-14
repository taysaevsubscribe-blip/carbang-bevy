#!/bin/sh
# Склейка частей и распаковка CarBang 0.2.0 для Linux
set -e
cd "$(dirname "$0")"
cat CarBang-0.2.0-linux-x86_64.tar.gz.part* > CarBang-0.2.0-linux-x86_64.tar.gz
sha256sum -c SHA256.txt
tar xzf CarBang-0.2.0-linux-x86_64.tar.gz
echo "Готово: ./CarBang-0.2.0-linux-x86_64/carbang.sh"
