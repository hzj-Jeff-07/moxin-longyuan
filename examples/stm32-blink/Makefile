# examples/stm32-blink/Makefile
#
# Standalone example build. cmd_build_stm32 调 arm-none-eabi-gcc 时
# 不走这个 Makefile —— 那条路径直接用 support/ 下的 startup.s + linker.ld
# 编 user 的 src/main.c。这个 Makefile 给"手动验证 example 能编"用。

CC      = arm-none-eabi-gcc
CFLAGS  = -mthumb -mcpu=cortex-m4 -mfloat-abi=soft \
          -Os -ffreestanding -nostartfiles -nostdlib \
          -Wall -Wextra
LDFLAGS = -Tsupport/linker.ld

OUT = blink.elf
SRCS = support/startup.s main.c

$(OUT): $(SRCS) support/linker.ld
	$(CC) $(CFLAGS) $(LDFLAGS) -o $@ $(SRCS)

clean:
	rm -f $(OUT)

.PHONY: clean
