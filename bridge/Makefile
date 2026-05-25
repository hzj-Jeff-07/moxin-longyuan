# Makefile for moxin-simavr-bridge
#
# 单独编译,不参与 cargo 构建,因为它链接 libsimavr.a (GPL-3.0+),
# 与 moxin Rust 主进程通过 stdio 隔离。
#
# Linux (CI / 用户机):
#   sudo apt install simavr-dev libelf-dev
#   make                    # 使用系统库 -lsimavr -lelf
#
# macOS (开发机):
#   make SIMAVR_SRC=/path/to/simavr-src
# 默认 SIMAVR_SRC = $(HOME)/projects/simavr-src

UNAME_S := $(shell uname -s)

ifeq ($(UNAME_S),Linux)
# ---------------- Linux: 链接系统 simavr-dev ----------------
SIMAVR_INC := -I/usr/include/simavr
SIMAVR_LIB_FLAGS := -lsimavr
SIMAVR_DEP :=
LIBELF_INC :=
LIBELF_LIB := -lelf
else
# ---------------- macOS: 链接源码构建的 libsimavr.a ----------------
SIMAVR_SRC ?= $(HOME)/projects/simavr-src
SIMAVR_LIB := $(SIMAVR_SRC)/simavr/obj-arm64-apple-darwin25.3.0/libsimavr.a
SIMAVR_INC := -I$(SIMAVR_SRC)/simavr/sim -I$(SIMAVR_SRC)/simavr/cores
SIMAVR_LIB_FLAGS := $(SIMAVR_LIB)
SIMAVR_DEP := $(SIMAVR_LIB)

LIBELF_PREFIX := $(shell brew --prefix libelf 2>/dev/null)
ifneq ($(LIBELF_PREFIX),)
LIBELF_INC := -I$(LIBELF_PREFIX)/include -I$(LIBELF_PREFIX)/include/libelf
LIBELF_LIB := -L$(LIBELF_PREFIX)/lib -lelf
endif
endif

CFLAGS  := -O2 -Wall -Wextra $(SIMAVR_INC) $(LIBELF_INC)
LDFLAGS := $(LIBELF_LIB) -lm

.PHONY: all clean

all: moxin-simavr-bridge

moxin-simavr-bridge: moxin-simavr-bridge.c $(SIMAVR_DEP)
	$(CC) $(CFLAGS) -o $@ $< $(SIMAVR_LIB_FLAGS) $(LDFLAGS)

clean:
	rm -f moxin-simavr-bridge
