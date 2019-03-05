arch ?= aarch64
mode ?= debug
target := $(arch)
payload ?=

bootloader := target/$(target)/$(mode)/rcore-bootloader

ifeq ($(arch), x86_64)
ifeq ($(uname), Darwin)
prefix := x86_64-elf-
endif
else ifeq ($(arch), riscv32)
prefix := riscv64-unknown-elf-
else ifeq ($(arch), riscv64)
prefix := riscv64-unknown-elf-
else ifeq ($(arch), aarch64)
prefix ?= aarch64-none-elf-
ifeq (,$(shell which $(prefix)ld))
	prefix := aarch64-elf-
endif
endif

ld := $(prefix)ld
objdump := $(prefix)objdump
objcopy := $(prefix)objcopy
cc := $(prefix)gcc
as := $(prefix)as
gdb := $(prefix)gdb
strip := $(prefix)strip

export CC = $(cc)
export PAYLOAD = $(payload)

build_args := --target=targets/$(arch).json
ifeq ($(mode), release)
build_args += --release
endif

.PHONY: all clean

all: bootloader

bootloader: $(payload)
	@cargo xbuild $(build_args)

clean:
	@cargo clean
