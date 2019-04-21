# Commands:
#   make build                  Build
#   make run                    Build and run in QEMU
#   make justrun                Run the last build
#   make runnet                 Build and run in QEMU with nic
#   make justrunnet             Run the last build with nic
#   make runui                  Build and run in QEMU with gui
#   make justrunui              Run the last build with gui
#   make runtest                Build and run in QEMU with specified program
#   make justruntest            Run the last build with specified program
#   make doc                    Generate docs
#   make asm                    Open the deassemble file of the last build
#   make header                 Open 'objdump -h' of the last build
#   make addr2line              Use addr2line to recover line info in backtrace
#   make clean                  Clean
#
# Options:
#   arch    = x86_64 | riscv32 | riscv64 | aarch64 | mipsel
#   d    = int | in_asm | ...   QEMU debug info
#   mode = debug | release
#   LOG  = off | error | warn | info | debug | trace
#   SFSIMG = <sfsimg>            SFS image path of user programs
#   smp     = 1 | 2 | ...        SMP core number
#   graphic = on | off           Enable/disable qemu graphical output
#   board   = none               Running on QEMU
#         | pc                   Only available on x86_64, run on real pc
#         | u540                 Only available on riscv64, run on HiFive U540, use Sv39
#         | k210                 Only available on riscv64, run on K210, use Sv39
#         | raspi3               Only available on aarch64, run on Raspberry Pi 3 Model B/B+
#   pci_passthru = 0000:00:00.1  Only available on x86_64, passthrough the specified PCI device
#   init = /bin/ls               Only available on riscv64, run specified program instead of user shell
#   extra_nic = on | off         Only available on x86_64, add an additional e1000 nic
#   u_boot = /path/to/u-boot.bin Only available on aarch64, use u-boot to boot rcore

arch ?= riscv64
board ?= none
mode ?= release
LOG  ?=
graphic ?= off
smp  ?= 4
pci_passthru ?=
init ?=
extra_nic ?= off

target := $(arch)
build_path := target/$(target)/$(mode)
kernel := $(build_path)/rcore
kernel_img := $(build_path)/kernel.img
bootimage := $(build_path)/bootimage.bin
bootloader_dir = ../bootloader
bootloader := $(bootloader_dir)/target/$(target)/$(mode)/rcore-bootloader
bbl_path := $(PWD)/../riscv-pk
user_dir := ../user


### export environments ###

ifeq ($(arch), $(filter $(arch), aarch64 mipsel))
#link user img, so use original image
export SFSIMG = $(user_dir)/build/$(arch).img
else
# board is pc or qemu?
ifeq ($(board), $(filter $(board), pc u540 k210))
#link user img, so use original image
export SFSIMG = $(user_dir)/build/$(arch).img
else
export SFSIMG = $(user_dir)/build/$(arch).qcow2
endif # pc or qemu
endif # aarch64 mipsel

ifeq ($(arch), aarch64)
board := raspi3
need_bootloader := true
endif

# currently only mipsel architecture needs DTB linked to the kernel
ifeq ($(arch), mipsel)
dtb := src/arch/$(arch)/board/$(board)/device.dtb
endif

# mipssim does not support SMP
ifeq ($(board), mipssim)
smp := 1
endif

export ARCH = $(arch)
export BOARD = $(board)
export SMP = $(smp)
export DTB = $(dtb)


### qemu options ###
qemu_opts := \
	-smp cores=$(smp)
qemu_net_opts :=

ifeq ($(arch), x86_64)
qemu_opts += \
	-drive format=raw,file=$(bootimage) \
	-serial mon:stdio \
	-m 4G \
	-device isa-debug-exit
ifeq ($(board), none)
qemu_opts += \
	-drive format=qcow2,file=$(SFSIMG),media=disk,cache=writeback,id=sfsimg,if=none \
	-device ahci,id=ahci0 \
	-device ide-drive,drive=sfsimg,bus=ahci0.0
endif
ifeq ($(pci_passthru), )
qemu_net_opts += \
	-netdev type=tap,id=net0,script=no,downscript=no \
	-device e1000e,netdev=net0
else
qemu_opts += \
	-machine accel=kvm
qemu_net_opts += \
	-device vfio-pci,host=$(pci_passthru)
endif
ifeq ($(extra_nic), on)
qemu_net_opts += \
	-netdev type=tap,id=net1,script=no,downscript=no \
	-device e1000e,netdev=net1
endif

else ifeq ($(arch), riscv32)
qemu_opts += \
	-machine virt \
	-kernel ../tools/opensbi/virt_rv32.elf \
	-device loader,addr=0x80400000,file=$(kernel_img) \
	-drive file=$(SFSIMG),format=qcow2,id=sfs \
	-device virtio-blk-device,drive=sfs
qemu_net_opts += \
	-netdev type=tap,id=net0,script=no,downscript=no \
	-device virtio-net-device,netdev=net0

else ifeq ($(arch), riscv64)
ifeq ($(board), u540)
qemu_opts += \
	-machine virt \
	-kernel ../tools/opensbi/fu540.elf \
	-device loader,addr=0x80200000,file=$(kernel_img)
else
qemu_opts += \
	-machine virt \
	-kernel ../tools/opensbi/virt_rv64.elf \
	-device loader,addr=0x80200000,file=$(kernel_img) \
	-drive file=$(SFSIMG),format=qcow2,id=sfs \
	-device virtio-blk-device,drive=sfs
endif
qemu_net_opts += \
	-netdev type=tap,id=net0,script=no,downscript=no \
	-device virtio-net-device,netdev=net0

else ifeq ($(arch), aarch64)
qemu_opts += \
	-machine $(board) \
	-serial null -serial mon:stdio \
	-kernel $(kernel_img)
ifneq ($(u_boot), )
qemu_opts += \
	-sd $(bootloader)
endif

else ifeq ($(arch), mipsel)
ifeq ($(board), malta)
qemu_opts += \
	-machine $(board) -device VGA \
	-serial null -serial null -serial mon:stdio \
	-kernel $(kernel_img)
endif
ifeq ($(board), mipssim)
qemu_opts += \
	-machine $(board) \
	-serial mon:stdio \
	-kernel $(kernel_img)
endif
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

ifneq ($(init), )
features += run_cmdline
endif

ifeq ($(board), raspi3)
# qemu only has generic timer
# TODO: configure system/generic timer automatically
raspi3_timer ?= generic
ifeq ($(raspi3_timer), generic)
features += raspi3_use_generic_timer
endif
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
else ifeq ($(arch), mipsel)
prefix ?= mipsel-linux-musln32-
else ifeq ($(arch), aarch64)
prefix ?= aarch64-none-elf-
ifeq (,$(shell which $(prefix)ld))
	prefix := aarch64-elf-
endif
endif

gdb := $(prefix)gdb
objdump := cargo objdump -- -arch-name=$(subst _,-,$(arch))
objcopy := cargo objcopy -- --binary-architecture=$(subst _,-,$(arch))
strip := cargo strip --
dtc := dtc
hostcc := gcc

.PHONY: all clean build asm doc debug kernel sfsimg install run justrun runnet justrunnet runui justrunui runtest justruntest

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
runtest: build justruntest

justrun:
	@qemu-system-$(arch) $(qemu_opts)

justrunnet: build
	@sudo qemu-system-$(arch) $(qemu_opts) $(qemu_net_opts)

justrunui: build
	@qemu-system-$(arch) $(qemu_opts) \
		-device virtio-gpu-device \
		-device virtio-mouse-device

justruntest: build
	@qemu-system-$(arch) $(qemu_opts) --append $(init) -serial file:../tests/stdout -monitor null

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

### device tree process ###

%.dtb: %.dts
	@echo Generating device tree file $@
	@$(dtc) -I dts -O dtb -o $@ $<
	@rm -rf src/arch/${arch}/boot/dtb.gen.s

### bootloader and kernel image ###

$(bootloader): $(kernel)
ifeq ($(need_bootloader), true)
	@echo Building $(arch) bootloader
	@$(strip) $(kernel) -o $(kernel)_stripped
	@cd $(bootloader_dir) && make arch=$(arch) mode=$(mode) payload=../kernel/$(kernel)_stripped
	@rm $(kernel)_stripped
endif

$(kernel_img): kernel $(bootloader)
ifeq ($(arch), $(filter $(arch), riscv32 riscv64))
	@$(objcopy) $(kernel) --strip-all -O binary $@
else ifeq ($(arch), aarch64)
ifneq ($(u_boot), )
	@cp $(u_boot) $@
else
	@$(objcopy) $(bootloader) --strip-all -O binary $@
endif
else ifeq ($(arch), mipsel)
# qemu-system-mipsel accepts ELF file only, so objcopy is not needed
	@$(strip) $(kernel) -o $@
endif

kernel: $(dtb)
	@echo Building $(arch) kernel
ifeq ($(arch), x86_64)
	@bootimage build $(build_args)
	@mv target/x86_64/bootimage.bin $(bootimage)
else ifeq ($(arch), $(filter $(arch), riscv32 riscv64))
ifeq ($(board), k210)
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
else ifeq ($(arch), mipsel)
	@for file in context entry trap ; do \
	    $(hostcc) -E src/arch/$(arch)/boot/$${file}.S -o src/arch/$(arch)/boot/$${file}.gen.s ; \
	done
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

ifeq ($(board), u540)
.PHONY:
install: $(kernel_img)
	@$(objcopy) -S -O binary ../tools/opensbi/fu540.elf $(build_path)/bin
	@dd if=$< of=$(build_path)/bin bs=0x20000 seek=16
	@../tools/u540/mkimg.sh $(build_path)/bin $(build_path)/u540.img
endif

ifeq ($(board), k210)
.PHONY:
install: $(kernel_img)
	@$(objcopy) -S -O binary ../tools/opensbi/k210.elf $(build_path)/k210.img
	@dd if=$< of=$(build_path)/k210.img bs=0x10000 seek=1
	@python3 ../tools/k210/kflash.py -b 600000 $(build_path)/k210.img
endif

.PHONY:
addr2line:
	@python3 ../tools/addr2line.py $(prefix)addr2line $(arch) $(mode)
