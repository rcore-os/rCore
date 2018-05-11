arch ?= x86_64
kernel := build/kernel-$(arch).bin
iso := build/os-$(arch).iso
target ?= $(arch)-blog_os
rust_os := target/$(target)/debug/libblog_os.a

boot_src := src/arch/$(arch)/boot
linker_script := $(boot_src)/linker.ld
grub_cfg := $(boot_src)/grub.cfg
assembly_source_files := $(wildcard $(boot_src)/*.asm)
assembly_object_files := $(patsubst $(boot_src)/%.asm, \
	build/arch/$(arch)/boot/%.o, $(assembly_source_files))
user_object_files := $(wildcard user/*.o)
qemu_opts := -cdrom $(iso) -smp 4 -serial mon:stdio
features := use_apic

link_user = 1

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

.PHONY: all clean run iso kernel build debug_asm

all: $(kernel)

clean:
	@rm -r build target

run: $(iso)
	@qemu-system-$(arch) $(qemu_opts) || [ $$? -eq 11 ] # run qemu and assert it exit 11

iso: $(iso)

build: iso

debug_asm:
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
	@RUST_TARGET_PATH=$(shell pwd) CC=$(cc) xargo build --target $(target) --features "$(features)"

# compile assembly files
build/arch/$(arch)/boot/%.o: $(boot_src)/%.asm
	@mkdir -p $(shell dirname $@)
	@nasm -felf64 $< -o $@

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
innerpwd ?= pwd
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