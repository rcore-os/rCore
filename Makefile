arch ?= x86_64
kernel := build/kernel-$(arch).bin
iso := build/os-$(arch).iso
target ?= $(arch)-blog_os
rust_os := target/$(target)/debug/libblog_os.a

linker_script := src/arch/$(arch)/linker.ld
grub_cfg := src/arch/$(arch)/grub.cfg
assembly_source_files := $(wildcard src/arch/$(arch)/*.asm)
assembly_object_files := $(patsubst src/arch/$(arch)/%.asm, \
	build/arch/$(arch)/%.o, $(assembly_source_files))

ifeq ($(shell uname), Linux)
	prefix :=
else
	prefix := x86_64-elf-
endif

ld := $(prefix)ld

.PHONY: all clean run iso kernel

all: $(kernel)

clean:
	@rm -r build

run: $(iso)
	@qemu-system-$(arch) -cdrom $(iso)

iso: $(iso)

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
	@RUST_TARGET_PATH=$(shell pwd) xargo build --target $(target)

# compile assembly files
build/arch/$(arch)/%.o: src/arch/$(arch)/%.asm
	@mkdir -p $(shell dirname $@)
	@nasm -felf64 $< -o $@

# used by docker_* targets
docker_image ?= blog_os
tag ?= 0.1
docker_cargo_volume ?=  blogos-$(shell id -u)-$(shell id -g)-cargo
docker_rustup_volume ?=  blogos-$(shell id -u)-$(shell id -g)-rustup
docker_args ?= -e LOCAL_UID=$(shell id -u) -e LOCAL_GID=$(shell id -g) -v $(docker_cargo_volume):/usr/local/cargo -v $(docker_rustup_volume):/usr/local/rustup -v $(shell pwd):$(shell pwd) -w $(shell pwd)
docker_clean_args ?= $(docker_cargo_volume) $(docker_rustup_volume)

# docker_* targets 

docker_build:
	@docker build docker/ -t $(docker_image):$(tag)

docker_iso: 
	@docker run --rm $(docker_args) $(docker_image):$(tag) make iso

docker_run: docker_iso
	@qemu-system-x86_64 -cdrom $(iso) -s

docker_interactive:
	@docker run -it --rm $(docker_args) $(docker_image):$(tag) 

docker_clean:
	@docker volume rm $(docker_clean_args)