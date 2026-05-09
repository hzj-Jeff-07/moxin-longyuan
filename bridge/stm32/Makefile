# bridge/stm32/Makefile · 编 bridge-stm32
#
# 不依赖 simavr / 任何嵌入式 SDK。只是 fork/exec qemu-system-arm + parse stdout。
# 跟 v1 的 simavr bridge 是平行 sibling,各自独立 Makefile。

CC      = cc
CFLAGS  = -O2 -Wall -Wextra -Wpedantic -std=c11 -D_POSIX_C_SOURCE=200809L
TARGET  = bridge-stm32
SRCS    = bridge-stm32.c

$(TARGET): $(SRCS)
	$(CC) $(CFLAGS) -o $@ $(SRCS)

clean:
	rm -f $(TARGET)

.PHONY: clean
