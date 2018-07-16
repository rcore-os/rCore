# Examples:
# 	make run					Run in debug
# 	make justrun				Run in debug without build
#	make run int=1				Run with interrupt info by QEMU
# 	make run mode=release		Run in release
#	make run LOG=error			Run with log level: error
#									LOG = off | error | warn | info | debug | trace
# 	make doc					Generate docs
# 	make asm					Open the deassemble file of the last build
# 	make clean					Clean

arch ?= riscv32
kernel := build/$(arch)/kernel.bin
iso := build/$(arch)/os.iso
target ?= $(arch)-blog_os
mode ?= debug
rust_lib := target/$(target)/$(mode)/librust_ucore.a

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
qemu_opts := -cdrom $(iso) -smp 4 -serial mon:stdio -drive file=$(SFSIMG),media=disk,cache=writeback
endif
ifeq ($(arch), riscv32)
qemu_opts := -machine virt -kernel $(iso) -nographic
endif
features := use_apic

LOG ?= debug

# Link user binaries at ../user
ifdef link_user
features := $(features) link_user_program
assembly_object_files := $(assembly_object_files) $(user_obj)
endif

# Link user-riscv.img for RV32
ifeq ($(arch), riscv32)
riscv_user_img_obj := build/riscv32/user-riscv.o
assembly_object_files := $(assembly_object_files) $(riscv_user_img_obj)
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

ifeq ($(uname), Darwin)
prefix := x86_64-elf-
endif
ifeq ($(arch), riscv32)
prefix := riscv32-unknown-elf-
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

build/riscv32/os.iso: $(kernel)
	@cd ../riscv-pk && \
	 mkdir -p build && \
	 cd build && \
	 ../configure \
	 	--enable-logo \
	 	--prefix=$(RISCV) \
	 	--disable-fp-emulation \
	 	--host=riscv32-unknown-elf \
	 	--with-payload=$(abspath $(kernel)) && \
	 make && \
	 cp bbl ../../kernel/$@

$(kernel): kernel $(assembly_object_files) $(linker_script)
	@$(ld) -n --gc-sections -T $(linker_script) -o $(kernel) \
		$(assembly_object_files) $(rust_lib)

kernel:
	@RUST_TARGET_PATH=$(shell pwd) CC=$(cc) xargo build $(build_args)

# compile assembly files
build/x86_64/boot/%.o: $(boot_src)/%.asm
	@mkdir -p $(shell dirname $@)
	@nasm -felf64 $< -o $@

build/riscv32/boot/%.o: $(boot_src)/%.asm
	@mkdir -p $(shell dirname $@)
	@$(as) -march=rv32i $< -o $@

# make user.o from binary files
$(user_obj): $(user_bins)
	@cd $(user_bin_path) && \
		$(ld) -o $(abspath $@) $(patsubst %, -b binary %, $(notdir $(user_bins)))

$(riscv_user_img_obj): ../user/user-riscv.img
	@cd ../user && $(ld) -o $(abspath $@) -b binary $(notdir $<)

# patch Rust core for RISCV32I atomic
patch-core:
	@patch -p0 /rust/rust-riscv-rust-1.26.0-1-dev/src/libcore/sync/atomic.rs src/arch/riscv32/atomic.patch