# Makefile for moxin-simavr-bridge
#
# 单独编译,不参与 cargo 构建,因为它链接 libsimavr.a (GPL-3.0+),
# 与 moxin Rust 主进程通过 stdio 隔离。
#
# 使用方式:
#   make SIMAVR_SRC=/path/to/simavr-src
# 默认 SIMAVR_SRC = ../../simavr-src

SIMAVR_SRC ?= $(HOME)/projects/simavr-src
SIMAVR_LIB := $(SIMAVR_SRC)/simavr/obj-arm64-apple-darwin25.3.0/libsimavr.a
SIMAVR_INC := -I$(SIMAVR_SRC)/simavr/sim -I$(SIMAVR_SRC)/simavr/cores

LIBELF_PREFIX := $(shell brew --prefix libelf 2>/dev/null)
ifneq ($(LIBELF_PREFIX),)
LIBELF_INC := -I$(LIBELF_PREFIX)/include -I$(LIBELF_PREFIX)/include/libelf
LIBELF_LIB := -L$(LIBELF_PREFIX)/lib -lelf
endif

CFLAGS  := -O2 -Wall -Wextra $(SIMAVR_INC) $(LIBELF_INC)
LDFLAGS := $(LIBELF_LIB) -lm

.PHONY: all clean

all: moxin-simavr-bridge

moxin-simavr-bridge: moxin-simavr-bridge.c $(SIMAVR_LIB)
	$(CC) $(CFLAGS) -o $@ $< $(SIMAVR_LIB) $(LDFLAGS)

clean:
	rm -f moxin-simavr-bridge
