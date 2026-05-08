#!/bin/bash
# scripts/build-stm32.sh · 一行命令编 examples/stm32-blink/blink.elf
#
# 这个脚本是给"手动验证 example"用,不走 cmd_build_stm32 路径
# (那条路径直接调 arm-none-eabi-gcc 不需要 Make)。

set -euo pipefail

cd "$(dirname "$0")/.."
make -C examples/stm32-blink/ clean
make -C examples/stm32-blink/

echo "✓ examples/stm32-blink/blink.elf"
ls -la examples/stm32-blink/blink.elf
