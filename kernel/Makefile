# Commands:
#   make build                  Build
#   make run                    Build and run in QEMU
#   make justrun                Run the last build
#   make test                   Build and run in QEMU with specified program
#   make justtest               Run the last build with specified program
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
#   USER_IMG = <sfsimg>          image path of user programs
#   smp     = 1 | 2 | ...        SMP core number
#   graphic = on | off | console Enable/disable qemu graphical output, or print console to graphic output
#   board   = none               Running on QEMU
#         | pc                   Only available on x86_64, run on real pc
#         | u540                 Only available on riscv64, run on HiFive U540, use Sv39
#         | k210                 Only available on riscv64, run on K210, use Sv39
#         | rocket_chip          Only available on riscv64, run on Rocket Chip, use Sv39
#         | raspi3               Only available on aarch64, run on Raspberry Pi 3 Model B/B+
#   net = on | off               Only available on x86_64, enable NIC
#   pci_passthru = 0000:00:00.1  Only available on x86_64, passthrough the specified PCI device
#   init = /bin/ls               Only available on riscv64, run specified program instead of user shell
#   extra_nic = on | off         Only available on x86_64, add an additional e1000 nic
# . extra_features = profile | ... Add additional features

arch ?= riscv64
board ?= none
mode ?= release
LOG  ?=
graphic ?= off
smp  ?= 4
pci_passthru ?=
init ?=
extra_nic ?= off

qemu := qemu-system-$(arch)
target := $(arch)
build_path := target/$(target)/$(mode)
kernel := $(build_path)/rcore
kernel_img := $(build_path)/kernel.img
ESP := $(build_path)/esp
OVMF := ../rboot/OVMF.fd
user_dir := ../user


### export environments ###

export USER_IMG = $(user_dir)/build/$(arch).img
export USER_QCOW2 = $(user_dir)/build/$(arch).qcow2

ifeq ($(arch), aarch64)
board := raspi3
endif

# currently only mipsel architecture and rocket-chip need DTB linked to the kernel
ifeq ($(arch), mipsel)
dtb := src/arch/$(arch)/board/$(board)/device.dtb
endif
ifeq ($(board), rocket_chip)
dtb := src/arch/riscv32/board/$(board)/device.dtb
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
	-drive if=pflash,format=raw,file=$(OVMF),readonly=on \
	-drive format=raw,file=fat:rw:$(ESP) \
	-serial mon:stdio \
	-m 4G \
	-device isa-debug-exit
ifeq ($(board), none)
qemu_opts += \
	-drive format=qcow2,file=$(USER_QCOW2),media=disk,cache=writeback,id=sfsimg,if=none \
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
qemu_ui_opts += \
	-vga std
endif
ifeq ($(extra_nic), on)
qemu_net_opts += \
	-netdev type=tap,id=net1,script=no,downscript=no \
	-device e1000e,netdev=net1
endif

else ifeq ($(arch), riscv32)
qemu_opts += \
	-machine virt \
	-serial mon:stdio \
	-kernel ../tools/opensbi/virt_rv32.elf \
	-device loader,addr=0x80400000,file=$(kernel_img) \
	-drive file=$(USER_QCOW2),format=qcow2,id=sfs \
	-device virtio-blk-device,drive=sfs \
	-device virtio-gpu-device \
	-device virtio-mouse-device
qemu_net_opts += \
	-netdev type=tap,id=net0,script=no,downscript=no \
	-device virtio-net-device,netdev=net0

else ifeq ($(arch), riscv64)
ifeq ($(board), u540)
qemu_opts += \
	-machine virt \
	-serial mon:stdio \
	-kernel ../tools/opensbi/fu540.elf \
	-device loader,addr=0x80200000,file=$(kernel_img)
else
qemu_opts += \
	-machine virt \
	-serial mon:stdio \
	-kernel ../tools/opensbi/virt_rv64.elf \
	-device loader,addr=0x80200000,file=$(kernel_img) \
	-drive file=$(USER_QCOW2),format=qcow2,id=sfs \
	-device virtio-blk-device,drive=sfs \
	-device virtio-gpu-device \
	-device virtio-mouse-device
endif
qemu_net_opts += \
	-netdev type=tap,id=net0,script=no,downscript=no \
	-device virtio-net-device,netdev=net0

else ifeq ($(arch), aarch64)
qemu_opts += \
	-machine $(board) \
	-smp 4 \
	-serial null -serial mon:stdio \
	-kernel $(kernel_img) \
	-drive file=$(USER_QCOW2),format=qcow2,id=sfs

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

ifeq ($(net), on)
qemu_opts += $(qemu_net_opts)
qemu := sudo $(qemu)
endif

### build args ###
ifeq ($(graphic), off)
features += nographic
endif

ifeq ($(graphic), console)
features += consolegraphic
endif

ifneq ($(init), )
features += run_cmdline
endif

ifneq ($(board), none)
features += board_$(board)
endif

features += $(extra_features)

build_args := --target targets/$(target).json --features "$(features)"

ifeq ($(mode), release)
build_args += --release
endif

### prefix ###
ifeq ($(arch), x86_64)
ifeq ($(uname), Darwin)
prefix := x86_64-linux-musl-
endif
else ifeq ($(arch), riscv32)
prefix := riscv64-linux-musl-
else ifeq ($(arch), riscv64)
prefix := riscv64-linux-musl-
else ifeq ($(arch), mipsel)
prefix ?= mipsel-linux-musln32-
else ifeq ($(arch), aarch64)
prefix ?= aarch64-linux-musl-
endif

gdb := gdb
objdump := cargo objdump $(build_args) -- -print-imm-hex
objcopy := cargo objcopy -- --binary-architecture=$(subst _,-,$(arch))
strip := cargo strip --
dtc := dtc
hostcc := gcc

.PHONY: all clean build asm doc debug kernel sfsimg install run justrun test justtest

all: kernel

clean:
	@cargo clean
	@cd $(user_dir) && make clean

doc:
	@cargo rustdoc -- --document-private-items

run: build justrun
test: build justtest

justrun:
	@$(qemu) $(qemu_opts)

justtest:
	# unavailable now
	@#$(qemu) $(filter-out -serial mon:stdio, $(qemu_opts)) --append $(init) -serial file:../tests/stdout -monitor null

debug: $(kernel) $(kernel_img)
	@$(qemu) $(qemu_opts) -s -S &
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

bootloader: $(kernel)
ifeq ($(arch), x86_64)
	@cd ../rboot && make build
endif

$(kernel_img): kernel bootloader
ifeq ($(arch), x86_64)
	@mkdir -p $(ESP)/EFI/rCore $(ESP)/EFI/Boot
	@cp ../rboot/target/x86_64-unknown-uefi/release/rboot.efi $(ESP)/EFI/Boot/BootX64.efi
	@cp ../rboot/rboot.conf $(ESP)/EFI/Boot/rboot.conf
	@cp $(kernel) $(ESP)/EFI/rCore/kernel.elf
else ifeq ($(arch), $(filter $(arch), riscv32 riscv64))
	@$(objcopy) $(kernel) --strip-all -O binary $@
else ifeq ($(arch), aarch64)
	@$(objcopy) $(kernel) --strip-all -O binary $@
else ifeq ($(arch), mipsel)
# qemu-system-mipsel accepts ELF file only, so objcopy is not needed
	@$(strip) $(kernel) -o $@
endif

kernel: $(dtb)
	@echo Building $(arch) kernel
ifeq ($(arch), x86_64)
	@cargo xbuild $(build_args)
else ifeq ($(arch), $(filter $(arch), riscv32 riscv64))
ifeq ($(board), k210)
	@cp src/arch/riscv32/board/k210/linker.ld src/arch/riscv32/boot/linker64.ld
else ifeq ($(board), rocket_chip)
	@cp src/arch/riscv32/board/rocket_chip/linker.ld src/arch/riscv32/boot/linker64.ld
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
	    $(hostcc) -Dboard_$(board) -E src/arch/$(arch)/boot/$${file}.S -o src/arch/$(arch)/boot/$${file}.gen.s ; \
	done
	$(hostcc) -Dboard_$(board) -E src/arch/$(arch)/boot/linker.ld.S -o src/arch/$(arch)/boot/linker.ld
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
	@python3.7 ../tools/addr2line.py $(prefix)addr2line $(arch) $(mode)
