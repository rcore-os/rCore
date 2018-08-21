# Commands:
#   make build					Build
# 	make run					Build and run in QEMU
# 	make justrun				Run the last build
# 	make doc					Generate docs
# 	make asm					Open the deassemble file of the last build
# 	make elf-h					Open 'objdump -h' of the last build
# 	make clean					Clean
#
# Options:
#   arch = x86_64 | riscv32
#   d    = int | in_asm | ...	QEMU debug info
#   mode = debug | release
#   LOG  = off | error | warn | info | debug | trace
#   board 						Only available on riscv32, build without bbl, run on board

arch ?= riscv32
mode ?= debug
LOG  ?= debug

kernel := build/$(arch)/kernel.bin
iso := build/$(arch)/os.iso
target := $(arch)-blog_os
rust_lib := target/$(target)/$(mode)/ucore

boot_src := src/arch/$(arch)/boot
linker_script := $(boot_src)/linker.ld
grub_cfg := $(boot_src)/grub.cfg
assembly_source_files := $(wildcard $(boot_src)/*.asm)
assembly_object_files := $(patsubst $(boot_src)/%.asm, \
	build/$(arch)/boot/%.o, $(assembly_source_files))
user_bin_path := ../user/target/$(arch)-ucore/debug
user_bins := $(patsubst $(user_bin_path)/%.d, $(user_bin_path)/%, $(wildcard $(user_bin_path)/*.d))
user_obj := build/$(arch)/user.o
SFSIMG := ../user/ucore32.img
ifeq ($(arch), x86_64)
qemu_opts := -cdrom $(iso) -smp 4 -serial mon:stdio -drive file=$(SFSIMG),media=disk,cache=writeback -device isa-debug-exit
endif
ifeq ($(arch), riscv32)
qemu_opts := -machine virt -kernel $(iso) -nographic
endif
features := use_apic

ifdef board
features := $(features) no_bbl
endif

# Link user binaries at ../user
ifdef link_user
features := $(features) link_user_program
assembly_object_files := $(assembly_object_files) $(user_obj)
endif

ifdef d
qemu_opts := $(qemu_opts) -d $(d)
endif

build_args := --target $(target).json --features "$(features)"

ifeq ($(mode), release)
build_args := $(build_args) --release
endif

ifeq ($(arch), x86_64)
build_args := $(build_args) --lib
endif


ifeq ($(OS),Windows_NT)
uname := Win32
else
uname := $(shell uname)
endif

ifeq ($(uname), Darwin)
prefix := x86_64-elf-
endif
ifeq ($(arch), riscv32)
prefix := riscv64-unknown-elf-
endif

ld := $(prefix)ld
objdump := $(prefix)objdump
cc := $(prefix)gcc
as := $(prefix)as

.PHONY: all clean run iso build asm doc justrun kernel

all: $(kernel)

clean:
	@rm -r build target

doc:
	@cargo rustdoc -- --document-private-items

run: $(iso) justrun

justrun:
	@qemu-system-$(arch) $(qemu_opts) || [ $$? -eq 11 ] # run qemu and assert it exit 11

debug: $(iso)
	@qemu-system-$(arch) $(qemu_opts) -s -S &

iso: $(iso)

build: iso

asm:
	@$(objdump) -dS $(kernel) | less

elf-h:
	@$(objdump) -h $(kernel)

build/x86_64/os.iso: $(kernel) $(grub_cfg)
	@mkdir -p build/isofiles/boot/grub
	@cp $(kernel) build/isofiles/boot/kernel.bin
	@cp $(grub_cfg) build/isofiles/boot/grub
	@grub-mkrescue -o $(iso) build/isofiles 2> /dev/null
	@rm -r build/isofiles

build/riscv32/os.iso: kernel
	@mkdir -p build/riscv32
	@cp $(rust_lib) $(kernel)
ifdef board
	@cp $(rust_lib) $@
else
	@cd ../riscv-pk && \
	 mkdir -p build && \
	 cd build && \
	 ../configure \
	 	--enable-32bit \
	 	--enable-logo \
	 	--disable-fp-emulation \
	 	--host=riscv64-unknown-elf \
	 	--with-payload=$(abspath $(rust_lib)) && \
	 make && \
	 cp bbl ../../kernel/$@
endif

$(kernel): kernel $(assembly_object_files) $(linker_script)
	@$(ld) -n --gc-sections -T $(linker_script) -o $(kernel) \
		$(assembly_object_files) target/x86_64-blog_os/$(mode)/libucore.a

kernel:
	@CC=$(cc) cargo xbuild $(build_args)

# compile assembly files
build/x86_64/boot/%.o: $(boot_src)/%.asm
	@mkdir -p $(shell dirname $@)
	@nasm -felf64 $< -o $@

# make user.o from binary files
$(user_obj): $(user_bins)
	@cd $(user_bin_path) && \
		$(ld) -o $(abspath $@) $(patsubst %, -b binary %, $(notdir $(user_bins)))
