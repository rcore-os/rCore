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
#   smp                         SMP core number
#   board 						Only available on riscv32, build without bbl, run on board

arch ?= riscv32
mode ?= debug
LOG  ?= debug
smp  ?= 4

target := $(arch)-blog_os
kernel := target/$(target)/$(mode)/ucore
bin := target/$(target)/$(mode)/kernel.bin
bootimage := target/$(target)/bootimage.bin

user_bin_path := ../user/target/$(arch)-ucore/debug
user_bins := $(patsubst $(user_bin_path)/%.d, $(user_bin_path)/%, $(wildcard $(user_bin_path)/*.d))
user_obj := build/$(arch)/user.o
SFSIMG := ../user/ucore32.img
ifeq ($(arch), x86_64)
qemu_opts := \
	-drive format=raw,file=$(bootimage) \
	-drive format=raw,file=$(SFSIMG),media=disk,cache=writeback \
	-smp cores=$(smp) \
	-serial mon:stdio \
	-device isa-debug-exit
endif
ifeq ($(arch), riscv32)
qemu_opts := \
	-machine virt \
	-kernel $(bin) \
	-nographic \
	-smp cores=$(smp)
endif

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

.PHONY: all clean run build asm doc justrun kernel

all: $(kernel)

clean:
	@cargo clean

doc:
	@cargo rustdoc -- --document-private-items

run: build justrun

justrun:
	@qemu-system-$(arch) $(qemu_opts) || [ $$? -eq 11 ] # run qemu and assert it exit 11

debug: $(bin)
	@qemu-system-$(arch) $(qemu_opts) -s -S &

ifeq ($(arch), x86_64)
build: kernel
else
build: $(bin)
endif

asm:
	@$(objdump) -dS $(kernel) | less

header:
	@$(objdump) -h $(kernel)

sym:
	@$(objdump) -t $(kernel) | less

$(bin): kernel
ifdef board
	@cp $(kernel) $@
else
	@cd ../riscv-pk && \
	 mkdir -p build && \
	 cd build && \
	 ../configure \
	 	--enable-32bit \
	 	--enable-logo \
	 	--disable-fp-emulation \
	 	--host=riscv64-unknown-elf \
	 	--with-payload=$(abspath $(kernel)) && \
	 make && \
	 cp bbl ../../kernel/$@
endif

kernel:
ifeq ($(arch), x86_64)
	@bootimage build $(build_args)
else
	@-patch -p0 -N -b \
		$(shell rustc --print sysroot)/lib/rustlib/src/rust/src/libcore/sync/atomic.rs \
		src/arch/riscv32/atomic.patch
	@CC=$(cc) cargo xbuild $(build_args)
endif

# make user.o from binary files
$(user_obj): $(user_bins)
	@cd $(user_bin_path) && \
		$(ld) -o $(abspath $@) $(patsubst %, -b binary %, $(notdir $(user_bins)))
