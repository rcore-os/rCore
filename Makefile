# Examples:
# 	make run					Run in debug
#	make run int=1				Run with interrupt info by QEMU
# 	make run mode=release		Run in release
#	make run LOG=error			Run with log level: error
#									LOG = off | error | warn | info | debug | trace
# 	make doc					Generate docs
# 	make asm					Open the deassemble file of the last build
# 	make clean					Clean

arch ?= x86_64
kernel := build/kernel-$(arch).bin
iso := build/os-$(arch).iso
target ?= $(arch)-blog_os
mode ?= debug
rust_os := target/$(target)/$(mode)/librust_ucore.a

boot_src := src/arch/$(arch)/boot
linker_script := $(boot_src)/linker.ld
grub_cfg := $(boot_src)/grub.cfg
assembly_source_files := $(wildcard $(boot_src)/*.asm)
assembly_object_files := $(patsubst $(boot_src)/%.asm, \
	build/arch/$(arch)/boot/%.o, $(assembly_source_files))
user_image_files := $(wildcard user/*.img)
user_object_files := $(patsubst user/%.img, build/user/%.o, $(user_image_files))
SFSIMG := user/ucore32.img
qemu_opts := -cdrom $(iso) -smp 4 -serial mon:stdio -drive file=$(SFSIMG),media=disk,cache=writeback
features := use_apic

LOG ?= debug

ifdef link_user
features := $(features) link_user_program
assembly_object_files := $(assembly_object_files) $(user_object_files)
endif

ifdef travis
test := 1
features := $(features) qemu_auto_exit
endif

ifdef test
features := $(features) test
# enable shutdown inside the qemu
qemu_opts := $(qemu_opts) -device isa-debug-exit
endif

ifdef int
qemu_opts := $(qemu_opts) -d int
endif

build_args := --target $(target) --features "$(features)"

ifeq ($(mode), release)
build_args := $(build_args) --release
endif


ifeq ($(OS),Windows_NT)
uname := Win32
else
uname := $(shell uname)
endif

ifeq ($(uname), Linux)
prefix :=
else
prefix := x86_64-elf-
endif

ld := $(prefix)ld
objdump := $(prefix)objdump
cc := $(prefix)gcc

.PHONY: all clean run iso kernel build asm doc

all: $(kernel)

clean:
	@rm -r build target

doc:
	@cargo rustdoc -- --document-private-items

run: $(iso)
	@qemu-system-$(arch) $(qemu_opts) || [ $$? -eq 11 ] # run qemu and assert it exit 11

debug: $(iso)
	@qemu-system-$(arch) $(qemu_opts) -s -S &

iso: $(iso)

build: iso

asm:
	@$(objdump) -dS $(kernel) | less

$(iso): $(kernel) $(grub_cfg)
	@mkdir -p build/isofiles/boot/grub
	@cp $(kernel) build/isofiles/boot/kernel.bin
	@cp $(grub_cfg) build/isofiles/boot/grub
	@grub-mkrescue -o $(iso) build/isofiles 2> /dev/null
	@rm -r build/isofiles

$(kernel): kernel $(rust_os) $(assembly_object_files) $(linker_script)
	@$(ld) -n --gc-sections -T $(linker_script) -o $(kernel) \
		$(assembly_object_files) $(rust_os)

kernel:
	@RUST_TARGET_PATH=$(shell pwd) CC=$(cc) xargo build $(build_args)

# compile assembly files
build/arch/$(arch)/boot/%.o: $(boot_src)/%.asm
	@mkdir -p $(shell dirname $@)
	@nasm -felf64 $< -o $@

# make .o from .img file
build/user/%.o: user/%.img
	@mkdir -p $(shell dirname $@)
	@$(ld) -r -b binary $< -o $@

# used by docker_* targets
docker_image ?= blog_os
tag ?= 0.1
pwd ?= $(realpath ./)
ifeq ($(OS),Windows_NT)
uid ?= 0
gid ?= 0
innerpwd ?= /root/blog_os
else
uid ?= $(shell id -u)
gid ?= $(shell id -g)
innerpwd ?= $(pwd)
endif
docker_cargo_volume ?=  blogos-$(uid)-$(gid)-cargo
docker_rustup_volume ?=  blogos-$(uid)-$(gid)-rustup
docker_args ?= -e LOCAL_UID=$(uid) -e LOCAL_GID=$(gid) -v $(docker_cargo_volume):/usr/local/cargo -v $(docker_rustup_volume):/usr/local/rustup -v $(pwd):$(innerpwd) -w $(innerpwd)
docker_clean_args ?= $(docker_cargo_volume) $(docker_rustup_volume)

# docker_* targets

docker_build:
	@docker build docker/ -t $(docker_image):$(tag)

docker_iso:
	docker run --rm $(docker_args) $(docker_image):$(tag) make iso

docker_run: docker_iso
	@qemu-system-$(arch) -cdrom $(iso) -s

docker_interactive:
	@docker run -it --rm $(docker_args) $(docker_image):$(tag)

docker_clean:
	@docker volume rm $(docker_clean_args)