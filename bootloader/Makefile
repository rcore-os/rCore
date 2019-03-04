arch ?= aarch64
mode ?= debug
target := $(arch)

bootloader := target/$(target)/$(mode)/rcore-bootloader

build_args := --target=targets/$(arch).json
ifeq ($(mode), release)
build_args += --release
endif

.PHONY: all clean

all: $(bootloader)

$(bootloader):
	@cargo xbuild $(build_args)

clean:
	@cargo clean
