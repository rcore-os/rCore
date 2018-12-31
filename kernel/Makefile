# Commands:
#   make build                  Build
#   make run                    Build and run in QEMU
#   make justrun                Run the last build
#   make doc                    Generate docs
#   make asm                    Open the deassemble file of the last build
#   make header                 Open 'objdump -h' of the last build
#   make clean                  Clean
#
# Options:
#   arch = x86_64 | riscv32 | riscv64 | aarch64
#   d    = int | in_asm | ...   QEMU debug info
#   mode = debug | release
#   LOG  = off | error | warn | info | debug | trace
#   SFSIMG = SFS image path of user programs
#   smp  = 1 | 2 | ...          SMP core number
#   board = none                Running on QEMU
#         | k210                Only available on riscv64, build without bbl, run on K210
#         | raspi3              Only available on aarch64, run on Raspberry Pi 3 Model B/B+
#   m_mode                      Only available on riscv32, build for M-Mode, without MMU

arch ?= riscv32
board ?= none
mode ?= debug
LOG  ?= debug
smp  ?= 4
m_mode ?=

target := $(arch)
kernel := target/$(target)/$(mode)/ucore
bin := target/$(target)/$(mode)/kernel.bin
bootimage := target/$(target)/bootimage.bin
bbl_path := $(PWD)/../riscv-pk
user_dir := ../user

### export environments ###
export ARCH = $(arch)
export SMP = $(smp)
#export SFSIMG = $(user_dir)/build/user-$(arch).img
ifeq ($(arch), x86_64)
export SFSIMG = $(user_dir)/img/ucore-i386.img
else
export SFSIMG = $(user_dir)/img/ucore-$(arch).img
endif
ifeq ($(arch), aarch64)
board := raspi3
endif
ifeq ($(board), k210)
m_mode := 1
endif
# crate 'process' use this to set interrupt (MIE or SIE)
export M_MODE = $(m_mode)

### qemu options ###
qemu_opts := \
	-smp cores=$(smp) \
	-nographic

ifeq ($(arch), x86_64)
qemu_opts += \
	-drive format=raw,file=$(bootimage) \
	-drive format=raw,file=$(SFSIMG),media=disk,cache=writeback \
	-serial mon:stdio \
	-device isa-debug-exit

else ifeq ($(arch), riscv32)
qemu_opts += \
	-machine virt \
	-kernel $(bin)
ifdef m_mode
qemu_opts += -cpu rv32imacu-nommu
endif

else ifeq ($(arch), riscv64)
qemu_opts += \
	-machine virt \
	-kernel $(bin)
ifdef m_mode
qemu_opts += -cpu rv64imacu-nommu
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
ifeq ($(board), raspi3)
# qemu only has generic timer
# TODO: configure system/generic timer automatically
raspi3_timer ?= generic
ifeq ($(raspi3_timer), generic)
features += raspi3_use_generic_timer
endif
endif

ifdef m_mode
features += no_mmu m_mode
bbl_m_mode := --enable-boot-machine
endif

ifneq ($(board), none)
features += board_$(board)
endif
build_args := --target targets/$(target).json --features "$(features)"

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
else ifeq ($(arch), riscv64)
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

.PHONY: all clean run build asm doc justrun debug kernel sfsimg install

all: kernel

clean:
	@cargo clean
	@cd $(user_dir) && make clean

doc:
	@cargo rustdoc -- --document-private-items

run: build justrun

justrun:
	@qemu-system-$(arch) $(qemu_opts)

debug: $(kernel) $(bin)
	@qemu-system-$(arch) $(qemu_opts) -s -S &
	@sleep 1
	@$(gdb) $(kernel) -x ../tools/gdbinit

build: $(bin)

asm:
	@$(objdump) -dS $(kernel) | less

header:
	@$(objdump) -h $(kernel)

sym:
	@$(objdump) -t $(kernel) | less

$(bin): kernel
ifeq ($(arch), riscv32)
	@mkdir -p target/$(target)/bbl && \
	cd target/$(target)/bbl && \
	$(bbl_path)/configure \
		$(bbl_m_mode) \
		--with-arch=rv32imac \
		--disable-fp-emulation \
		--host=riscv64-unknown-elf \
		--with-payload=$(abspath $(kernel)) && \
	make -j32 && \
	cp bbl $(abspath $@)
else ifeq ($(arch), riscv64)
ifeq ($(board), k210)
	@$(objcopy) $(kernel) --strip-all -O binary $@
else
	@mkdir -p target/$(target)/bbl && \
	cd target/$(target)/bbl && \
	$(bbl_path)/configure \
		$(bbl_m_mode) \
		--with-arch=rv64imac \
		--disable-fp-emulation \
		--host=riscv64-unknown-elf \
		--with-payload=$(abspath $(kernel)) && \
	make -j32 && \
	cp bbl $(abspath $@)
endif
else ifeq ($(arch), aarch64)
	@$(objcopy) $(kernel) --strip-all -O binary $@
endif

kernel:
ifeq ($(arch), x86_64)
	@bootimage build $(build_args)
else ifeq ($(arch), riscv32)
	@-patch -p0 -N -b \
		$(shell rustc --print sysroot)/lib/rustlib/src/rust/src/libcore/sync/atomic.rs \
		src/arch/riscv32/atomic.patch
	@CC=$(cc) cargo xbuild $(build_args)
else ifeq ($(arch), riscv64)
	@./build-rv64
else ifeq ($(arch), aarch64)
	@CC=$(cc) cargo xbuild $(build_args)
endif


### user programs ###
sfsimg:
	@cd $(user_dir) && make sfsimg


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

ifeq ($(board), k210)
.PHONY:
install: $(bin)
	## baudrate no more than 600000
	@python3 ../tools/k210/kflash.py $(bin) -b 600000
endif