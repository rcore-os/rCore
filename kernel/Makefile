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
#   ARCH = x86_64 | riscv32 | riscv64 | aarch64 | mipsel
#   D    = int | in_asm | ...   QEMU debug info
#   MODE = debug | release
#   LOG  = off | error | warn | info | debug | trace
#   USER_IMG = <sfsimg>         Image path of user programs
#   SMP  = 1 | 2 | ...          SMP core number
#   GRAPHIC = on | off | console Enable/disable qemu graphical output, or print console to graphic output
#   BOARD = qemu                Run on QEMU
#         | pc                  [ x86_64 only] Run on real pc
#         | u540                [riscv64 only] Run on HiFive U540, use Sv39
#         | raspi3              [aarch64 only] Run on Raspberry Pi 3 Model B/B+
#         | rcore_vmm_guest     [riscv64 only] Run on rust-rvm-vmm/RVM. Requires variable GUEST_USER_IMG to be specified.
#   NET = on | off              [ x86_64 only] Enable NIC
#   PCI_PASSTHRU = 0000:00:00.1 [ x86_64 only] Passthrough the specified PCI device
#   INIT = /bin/ls              [riscv64 only] Run specified program instead of user shell
#   EXTRA_NIC = on | off        [ x86_64 only] Add an additional e1000 nic
#   ACCEL = on | off            [ x86_64 only] Enable/disable kvm/hvf acceleration
#   HYPERVISOR = on | off       [ x86_64 and riscv64 only] Enable/disable the RVM hypervisor, and set ACCEL to on under x86_64
#   UART2 = on | off            [riscv64 only] Add an extra virtio-driven UART port on unix domain socket /tmp/rcore_uart2
#   GUEST_USER_IMG = <sfsimg>   Image path of user programs. Specially taken out to allow out-of-tree user image.
#   FEATURES = profile | ...    Add additional features

ARCH ?= riscv64
MODE ?= release
LOG  ?=
GRAPHIC ?= off
SMP  ?= 4
PCI_PASSTHRU ?=
INIT ?=
EXTRA_NIC ?= off
ACCEL ?= off
HYPERVISOR ?= off
UART2 ?= off

qemu := qemu-system-$(ARCH)
target := $(ARCH)
build_path := target/$(target)/$(MODE)
kernel := $(build_path)/rcore
kernel_img := $(build_path)/kernel.img
ESP := $(build_path)/esp
OVMF := ../rboot/OVMF.fd
user_dir := ../user


### export environments ###

ifeq ($(BOARD), rcore_vmm_guest)
ifeq ($(GUEST_USER_IMG), )
$(error For building guest rCore image you have to specify the variable GUEST_USER_IMG .)
endif
export USER_IMG = $(GUEST_USER_IMG)
else
export USER_IMG = $(user_dir)/build/$(ARCH).img
endif
export USER_QCOW2 = $(user_dir)/build/$(ARCH).qcow2

ifeq ($(ARCH), aarch64)
BOARD ?= raspi3
else ifeq ($(ARCH), mipsel)
BOARD ?= malta
else
BOARD ?= qemu
endif

# currently only mipsel architecture need DTB linked to the kernel
ifeq ($(ARCH), mipsel)
DTB := src/arch/$(ARCH)/board/$(BOARD)/device.dtb
endif

### qemu options ###
qemu_opts := \
	-smp cores=$(SMP)
qemu_net_opts :=

ifeq ($(ARCH), x86_64)
qemu_opts += \
	-drive if=pflash,format=raw,readonly,file=$(OVMF) \
	-drive format=raw,file=fat:rw:$(ESP) \
	-serial mon:stdio \
	-m 4G \
	-device isa-debug-exit
ifeq ($(BOARD), qemu)
qemu_opts += \
	-drive format=qcow2,file=$(USER_QCOW2),media=disk,cache=writeback,id=sfsimg,if=none \
	-device ahci,id=ahci0 \
	-device ide-hd,drive=sfsimg,bus=ahci0.0
endif
ifeq ($(PCI_PASSTHRU), )
qemu_net_opts += \
	-netdev type=tap,id=net0,script=no,downscript=no \
	-device e1000e,netdev=net0
else
qemu_net_opts += \
	-device vfio-pci,host=$(PCI_PASSTHRU)
qemu_ui_opts += \
	-vga std
endif
ifeq ($(EXTRA_NIC), on)
qemu_net_opts += \
	-netdev type=tap,id=net1,script=no,downscript=no \
	-device e1000e,netdev=net1
endif

else ifeq ($(ARCH), riscv32)
qemu_opts += \
	-machine virt \
	-serial mon:stdio \
	-bios default \
	-device loader,addr=0x80400000,file=$(kernel_img) \
	-drive file=$(USER_QCOW2),format=qcow2,id=sfs \
	-device virtio-blk-device,drive=sfs \
	-device virtio-gpu-device \
	-device virtio-mouse-device
qemu_net_opts += \
	-netdev type=tap,id=net0,script=no,downscript=no \
	-device virtio-net-device,netdev=net0

else ifeq ($(ARCH), riscv64)
ifeq ($(BOARD), u540)
qemu_opts += \
	-machine virt \
	-serial mon:stdio \
	-bios ../tools/opensbi/fu540.elf \
	-device loader,addr=0x80200000,file=$(kernel_img)
else ifeq ($(BOARD), rcore_vmm_guest)
qemu_opts += \
	-machine virt \
	-cpu rv64,x-h=true \
	-serial mon:stdio \
	-bios ../tools/opensbi/fw_jump.elf \
	-m 1G\
	-kernel $(kernel_img)
else
qemu_opts += \
	-machine virt \
	-cpu rv64,x-h=true \
	-m 2G \
	-serial mon:stdio
ifeq ($(UART2), on)
qemu_opts += \
-chardev socket,path=/tmp/rcore_uart2,server=on,wait=off,id=uart2 \
-device virtio-serial-device,id=uart2-bus \
-device virtconsole,chardev=uart2
endif
qemu_opts += \
	-bios ../tools/opensbi/fw_jump.elf  \
	-device loader,addr=0x80200000,file=$(kernel_img) \
	-drive file=$(USER_QCOW2),format=qcow2,id=sfs \
	-device virtio-blk-device,drive=sfs \
	-device virtio-gpu-device \
	-device virtio-mouse-device
endif
qemu_net_opts += \
	-netdev type=tap,id=net0,script=no,downscript=no \
	-device virtio-net-device,netdev=net0

else ifeq ($(ARCH), aarch64)
# raspi must have at least 4 cpus
qemu_opts += \
	-machine $(BOARD) \
	-smp 4 \
	-serial null -serial mon:stdio \
	-kernel $(kernel_img) \
	-drive file=$(USER_QCOW2),if=sd,format=qcow2,id=sfs

else ifeq ($(ARCH), mipsel)
ifeq ($(BOARD), malta)
qemu_opts += \
	-machine $(BOARD) -device VGA \
	-hda $(USER_QCOW2) \
	-serial null -serial null -serial mon:stdio \
	-kernel $(kernel_img)
endif
endif

ifdef D
qemu_opts += -d $(D)
endif

ifeq ($(GRAPHIC), off)
qemu_opts += -nographic
endif

ifeq ($(NET), on)
qemu_opts += $(qemu_net_opts)
qemu := sudo $(qemu)
endif

ifeq ($(HYPERVISOR), on)
FEATURES += hypervisor
ifeq ($(ARCH), x86_64)
ACCEL = on
endif
endif

ifeq ($(ACCEL), on)
ifeq ($(shell uname), Darwin)
qemu_opts += -accel hvf
else
qemu_opts += -accel kvm -cpu host
endif
endif

### build args ###
ifeq ($(GRAPHIC), off)
FEATURES += nographic
endif

ifeq ($(GRAPHIC), console)
FEATURES += consolegraphic
endif

ifneq ($(INIT), )
FEATURES += run_cmdline
endif

FEATURES += board_$(BOARD)

build_args := \
	-Z build-std=core,alloc \
	--target targets/$(target).json \
	--features "$(FEATURES)"

ifeq ($(MODE), release)
build_args += --release
endif

### prefix ###
ifeq ($(ARCH), x86_64)
ifeq ($(shell uname), Darwin)
prefix := x86_64-linux-musl-
endif
else ifeq ($(ARCH), riscv32)
prefix := riscv64-linux-musl-
else ifeq ($(ARCH), riscv64)
prefix := riscv64-linux-musl-
else ifeq ($(ARCH), mipsel)
prefix ?= mipsel-linux-musln32-
else ifeq ($(ARCH), aarch64)
prefix ?= aarch64-linux-musl-
endif

gdb := gdb
sysroot := $(shell rustc --print sysroot)
objdump := $(shell find $(sysroot) -name llvm-objdump) -print-imm-hex
objcopy := $(shell find $(sysroot) -name llvm-objcopy)
strip := $(shell find $(sysroot) -name llvm-strip)
dtc := dtc
hostcc := gcc

.PHONY: all clean build asm doc debug kernel sfsimg install run justrun test justtest

all: kernel

clean:
	@cargo clean
	@cd $(user_dir) && make clean

doc:
	@cargo doc $(build_args) --no-deps --document-private-items

run: build justrun
test: build justtest

justrun:
	@$(qemu) $(qemu_opts)

justtest:
	# unavailable now
	@#$(qemu) $(filter-out -serial mon:stdio, $(qemu_opts)) --append $(INIT) -serial file:../tests/stdout -monitor null

debug: $(kernel) $(kernel_img)
	@$(qemu) $(qemu_opts) -s -S &
	@sleep 1
	@$(gdb) $(kernel) -x ../tools/gdbinit

justdebug:
	@$(qemu) $(qemu_opts) -s -S
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
ifeq ($(ARCH), x86_64)
	@cd ../rboot && make build
endif

$(kernel_img): kernel bootloader
ifeq ($(ARCH), x86_64)
	mkdir -p $(ESP)/EFI/rCore $(ESP)/EFI/Boot
	@cp ../rboot/target/x86_64-unknown-uefi/$(MODE)/rboot.efi $(ESP)/EFI/Boot/BootX64.efi
	@cp ../rboot/rboot.conf $(ESP)/EFI/Boot/rboot.conf
	@cp $(kernel) $(ESP)/EFI/rCore/kernel.elf
else ifeq ($(ARCH), $(filter $(ARCH), riscv32 riscv64))
	@$(objcopy) $(kernel) --strip-all -O binary $@
else ifeq ($(ARCH), aarch64)
	@$(objcopy) $(kernel) --strip-all -O binary $@
else ifeq ($(ARCH), mipsel)
# qemu-system-mipsel accepts ELF file only, so objcopy is not needed
	@$(strip) $(kernel) -o $@
endif

kernel: $(DTB)
	@echo Building $(ARCH) kernel
ifeq ($(ARCH), $(filter $(ARCH), riscv32 riscv64))
	@cp src/arch/riscv/board/u540/linker.ld src/arch/riscv/boot/linker64.ld
	@-patch -p0 -N -b \
		$(sysroot)/lib/rustlib/src/rust/src/libcore/sync/atomic.rs \
		src/arch/riscv/atomic.patch
else ifeq ($(ARCH), mipsel)
	@for file in entry ; do \
	    $(hostcc) -Dboard_$(BOARD) -E src/arch/$(ARCH)/boot/$${file}.S -o src/arch/$(ARCH)/boot/$${file}.gen.s ; \
	done
	$(hostcc) -Dboard_$(BOARD) -E src/arch/$(ARCH)/boot/linker.ld.S -o src/arch/$(ARCH)/boot/linker.ld
endif
	@cargo build $(build_args)


### user programs ###
sfsimg:
	@cd $(user_dir) && make sfsimg


### install ###
ifeq ($(BOARD), raspi3)
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

ifeq ($(BOARD), u540)
.PHONY:
install: $(kernel_img)
	@$(objcopy) -S -O binary ../tools/opensbi/fu540.elf $(build_path)/bin
	@dd if=$< of=$(build_path)/bin bs=0x20000 seek=16
	@../tools/u540/mkimg.sh $(build_path)/bin $(build_path)/u540.img
endif

.PHONY:
addr2line:
	@python3 ../tools/addr2line.py $(prefix)addr2line $(ARCH) $(MODE)
