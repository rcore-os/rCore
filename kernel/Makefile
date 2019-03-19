# Commands:
#   make build                  Build
#   make run                    Build and run in QEMU
#   make justrun                Run the last build
#   make doc                    Generate docs
#   make asm                    Open the deassemble file of the last build
#   make header                 Open 'objdump -h' of the last build
#   make addr2line              Use addr2line to recover line info in backtrace
#   make clean                  Clean
#
# Options:
#   arch    = x86_64 | riscv32 | riscv64 | aarch64
#   d    = int | in_asm | ...   QEMU debug info
#   mode = debug | release
#   LOG  = off | error | warn | info | debug | trace
#   SFSIMG = <sfsimg>              SFS image path of user programs
#   smp     = 1 | 2 | ...           SMP core number
#   graphic = on | off              enable/disable qemu graphical output
#   board   = none                Running on QEMU
#         | k210                Only available on riscv64, build without bbl, run on K210
#         | u540                Only available on riscv64, run on HiFive U540, use Sv39
#         | raspi3              Only available on aarch64, run on Raspberry Pi 3 Model B/B+
#   m_mode                      Only available on riscv32, build for M-Mode, without MMU

arch ?= riscv64
board ?= none
mode ?= debug
LOG  ?= debug
graphic ?= off
smp  ?= 4
m_mode ?=

target := $(arch)
build_path := target/$(target)/$(mode)
kernel := $(build_path)/rcore
kernel_img := $(build_path)/kernel.img
bootimage := $(build_path)/bootimage.bin
bootloader_dir = ../bootloader
bootloader := $(bootloader_dir)/target/$(target)/$(mode)/rcore-bootloader
bbl_path := $(PWD)/../riscv-pk
user_dir := ../user
k210_lib := ../tools/k210/libkendryte.a

### export environments ###
export ARCH = $(arch)
export BOARD = $(board)
export SMP = $(smp)
export SFSIMG = $(user_dir)/build/$(arch).img
ifeq ($(arch), aarch64)
board := raspi3
endif
ifeq ($(board), k210)
m_mode := 1
endif
# crate 'process' use this to set interrupt (MIE or SIE)
export M_MODE = $(m_mode)

ifeq ($(arch), aarch64)
graphic := on
endif

### qemu options ###
qemu_opts := \
	-smp cores=$(smp)
qemu_net_opts := \
	-netdev type=tap,id=net0,script=no,downscript=no

ifeq ($(arch), x86_64)
qemu_opts += \
	-drive format=raw,file=$(bootimage) \
	-drive format=raw,file=$(SFSIMG),media=disk,cache=writeback \
	-serial mon:stdio \
	-device isa-debug-exit
qemu_net_opts += \
	-device e1000e,netdev=net0

else ifeq ($(arch), riscv32)
qemu_opts += \
	-machine virt \
	-kernel $(kernel_img) \
	-drive file=$(SFSIMG),format=raw,id=sfs \
	-device virtio-blk-device,drive=sfs
qemu_net_opts += \
	-device virtio-net-device,netdev=net0

ifdef m_mode
qemu_opts += -cpu rv32imacu-nommu
endif

else ifeq ($(arch), riscv64)
qemu_opts += \
	-machine virt \
	-kernel $(kernel_img) \
	-drive file=$(SFSIMG),format=raw,id=sfs \
	-device virtio-blk-device,drive=sfs
qemu_net_opts += \
	-device virtio-net-device,netdev=net0

ifdef m_mode
qemu_opts += -cpu rv64imacu-nommu
endif

else ifeq ($(arch), aarch64)
qemu_opts += \
	-machine $(board) \
	-serial null -serial mon:stdio \
	-kernel $(kernel_img)
endif

ifdef d
qemu_opts += -d $(d)
endif

ifeq ($(graphic), off)
qemu_opts += -nographic
endif

### build args ###
ifneq ($(graphic), on)
features += nographic
endif

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
riscv_pk_args := --enable-boot-machine
endif

ifeq ($(board), u540)
features += sv39
riscv_pk_args += --enable-sv39
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

.PHONY: all clean run build asm doc justrun debug kernel sfsimg install runnet

all: kernel

clean:
	@cargo clean
	@cd $(bootloader_dir) && make clean
	@cd $(user_dir) && make clean

doc:
	@cargo rustdoc -- --document-private-items

run: build justrun
runnet: build justrunnet
runui: build justrunui

justrun:
	@qemu-system-$(arch) $(qemu_opts)

justrunnet: build
	@sudo qemu-system-$(arch) $(qemu_opts) $(qemu_net_opts)

justrunui: build
	@qemu-system-$(arch) $(qemu_opts) \
		-device virtio-gpu-device \
		-device virtio-mouse-device

debug: $(kernel) $(kernel_img)
	@qemu-system-$(arch) $(qemu_opts) -s -S &
	@sleep 1
	@$(gdb) $(kernel) -x ../tools/gdbinit

build: $(kernel_img)

asm:
	@$(objdump) -d $(kernel) | less

header:
	@$(objdump) -h $(kernel)

sym:
	@$(objdump) -t $(kernel) | less

$(bootloader): $(kernel)
ifeq ($(arch), aarch64)
	@echo Building $(arch) bootloader
	@$(strip) $(kernel) -o $(kernel)_stripped
	@cd $(bootloader_dir) && make arch=$(arch) mode=$(mode) payload=../kernel/$(kernel)_stripped
	@rm $(kernel)_stripped
endif

$(kernel_img): kernel $(bootloader)
ifeq ($(arch), riscv32)
	@mkdir -p target/$(target)/bbl && \
	cd target/$(target)/bbl && \
	$(bbl_path)/configure \
		$(riscv_pk_args) \
		--with-arch=rv32imac \
		--disable-fp-emulation \
		--host=riscv64-unknown-elf \
		--with-payload=$(abspath $(kernel)) && \
	make -j && \
	cp bbl $(abspath $@)
else ifeq ($(arch), riscv64)
ifeq ($(board), k210)
	@$(objcopy) $(kernel) --strip-all -O binary $@
else
	@mkdir -p target/$(target)/bbl && \
	cd target/$(target)/bbl && \
	$(bbl_path)/configure \
		$(riscv_pk_args) \
		--with-arch=rv64imac \
		--disable-fp-emulation \
		--host=riscv64-unknown-elf \
		--with-payload=$(abspath $(kernel)) && \
	make -j && \
	cp bbl $(abspath $@)
endif
else ifeq ($(arch), aarch64)
	@$(objcopy) $(bootloader) --strip-all -O binary $@
endif

kernel:
	@echo Building $(arch) kernel
ifeq ($(arch), x86_64)
	@bootimage build $(build_args)
	@mv target/x86_64/bootimage.bin $(bootimage)
else ifeq ($(arch), riscv32)
	@-patch -p0 -N -b \
		$(shell rustc --print sysroot)/lib/rustlib/src/rust/src/libcore/sync/atomic.rs \
		src/arch/riscv32/atomic.patch
	@cargo xbuild $(build_args)
else ifeq ($(arch), riscv64)
ifeq ($(board), k210)
	@[[ -f $(k210_lib) ]] || wget https://github.com/wangrunji0408/RustOS/releases/download/v0.1/libkendryte.a -O $(k210_lib)
	@cp src/arch/riscv32/board/k210/linker.ld src/arch/riscv32/boot/linker64.ld
else
	@cp src/arch/riscv32/board/u540/linker.ld src/arch/riscv32/boot/linker64.ld
endif
	@-patch -p0 -N -b \
		$(shell rustc --print sysroot)/lib/rustlib/src/rust/src/libcore/sync/atomic.rs \
		src/arch/riscv32/atomic.patch
	@cargo xbuild $(build_args)
else ifeq ($(arch), aarch64)
	@cargo xbuild $(build_args)
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
install: $(kernel_img)
	cp $(kernel_img) $(sd_card)/kernel8.img
	sudo umount $(sd_card)
endif

endif

ifeq ($(board), k210)
.PHONY:
install: $(kernel_img)
	## baudrate no more than 600000
	@python3 ../tools/k210/kflash.py $(kernel_img) -b 600000
endif

ifeq ($(board), u540)
.PHONY:
install: $(kernel_img)
	@$(objcopy) -S -O binary --change-addresses -0x80000000 $< $(build_path)/bin
	@../tools/u540/mkimg.sh $(build_path)/bin $(build_path)/sd.img
endif

.PHONY:
addr2line:
	@python3 ../tools/addr2line.py $(prefix)addr2line $(arch)
