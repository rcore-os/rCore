# Commands:
#   make build					Build
# 	make run					Build and run in QEMU
# 	make justrun				Run the last build
# 	make doc					Generate docs
# 	make asm					Open the deassemble file of the last build
# 	make header					Open 'objdump -h' of the last build
# 	make clean					Clean
#
# Options:
#   arch = x86_64 | riscv32 | aarch64
#   d    = int | in_asm | ...	QEMU debug info
#   mode = debug | release
#   LOG  = off | error | warn | info | debug | trace
#   smp  = 1 | 2 | ...          SMP core number
#   board = fpga				Only available on riscv32, build without bbl, run on board
#         | raspi3				Only available on aarch64, run on Raspberry Pi 3 Model B/B+
#   nommu						Only available on riscv32, build for M-Mode, without MMU

arch ?= riscv32
board ?= raspi3
mode ?= debug
LOG  ?= debug
smp  ?= 4
nommu ?=

target := $(arch)-blog_os
kernel := target/$(target)/$(mode)/ucore
bin := target/$(target)/$(mode)/kernel.bin
bootimage := target/$(target)/bootimage.bin

### qemu options ###
qemu_opts := \
	-smp cores=$(smp) \
	-nographic

ifeq ($(arch), x86_64)
qemu_opts += \
	-drive format=raw,file=$(bootimage) \
	-drive format=raw,file="../user/img/ucore-i386-pic.img",media=disk,cache=writeback \
	-serial mon:stdio \
	-device isa-debug-exit

else ifeq ($(arch), riscv32)
qemu_opts += \
	-machine virt \
	-kernel $(bin)
ifdef nommu
qemu_opts += -cpu rv32imacu-nommu
endif

else ifeq ($(arch), aarch64)
qemu_opts += \
	-machine $(board) \
	-serial null -serial mon:stdio \
	-kernel $(bin)
endif

ifdef d
qemu_opts += -d $(d)
endif

### build args ###
ifeq ($(arch), riscv32)
ifeq ($(board), fpga)
features += no_bbl
endif
endif

ifdef nommu
features += no_mmu
bbl_m_mode := --enable-boot-machine
endif

features += board_$(board)
build_args := --target $(target).json --features "$(features)"

ifeq ($(mode), release)
build_args += --release
endif



### prefix ###
ifeq ($(arch), x86_64)
ifeq ($(uname), Darwin)
prefix := x86_64-elf-
endif
else ifeq ($(arch), riscv32)
prefix := riscv64-unknown-elf-
else ifeq ($(arch), aarch64)
prefix ?= aarch64-none-elf-
endif

ld := $(prefix)ld
objdump := $(prefix)objdump
objcopy := $(prefix)objcopy
cc := $(prefix)gcc
as := $(prefix)as
gdb := $(prefix)gdb

.PHONY: all clean run build asm doc justrun kernel

all: kernel

clean:
	@cargo clean
	@rm -rf ../riscv-pk/build

doc:
	@cargo rustdoc -- --document-private-items

run: build justrun

justrun:
	@qemu-system-$(arch) $(qemu_opts) || [ $$? -eq 11 ] # run qemu and assert it exit 11

debug: $(kernel) $(bin)
	@qemu-system-$(arch) $(qemu_opts) -s -S &
	@sleep 1
	@$(gdb) $(kernel) -x ../tools/gdbinit

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
ifeq ($(arch), riscv32)
ifeq ($(board), fpga)
	@cp $(kernel) $@
else
	@cd ../riscv-pk && \
	mkdir -p build && \
	cd build && \
	../configure \
		$(bbl_m_mode) \
		--with-arch=rv32imac \
		--disable-fp-emulation \
		--host=riscv64-unknown-elf \
		--with-payload=$(abspath $(kernel)) && \
	make && \
	cp bbl ../../kernel/$@
endif
else ifeq ($(arch), aarch64)
	@$(objcopy) $(kernel) --strip-all -O binary $@
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


### install ###
ifeq ($(board), raspi3)
sd_card ?=

ifeq ($(shell uname), Darwin)
sd_card := /Volumes/boot
else ifeq ($(shell uname), Linux)
sd_card := /media/$(shell whoami)/boot
endif

ifdef sd_card
.PHONY:
install: $(bin)
	cp $(bin) $(sd_card)/kernel8.img
	sudo umount $(sd_card)
endif

endif
