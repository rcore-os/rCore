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
#   arch = x86_64 | riscv32 | aarch64
#   d    = int | in_asm | ...	QEMU debug info
#   mode = debug | release
#   LOG  = off | error | warn | info | debug | trace
#   board = fpga				Only available on riscv32, build without bbl, run on board
#         | raspi3				Only available on aarch64, run on Raspberry Pi 3 Model B/B+

arch ?= aarch64
board ?= raspi3
prefix ?= $(arch)-none-elf
mode ?= debug
LOG  ?= debug

target := $(arch)-blog_os
kernel := target/$(target)/$(mode)/ucore
bin := target/$(target)/$(mode)/kernel.bin
bootimage := target/$(target)/bootimage.bin

user_bin_path := ../user/target/$(arch)-ucore/debug
user_bins := $(patsubst $(user_bin_path)/%.d, $(user_bin_path)/%, $(wildcard $(user_bin_path)/*.d))
user_obj := build/$(arch)/user.o
SFSIMG := ../user/ucore32.img

### qemu options ###
ifeq ($(arch), x86_64)
qemu_opts := \
	-drive format=raw,file=$(bootimage) \
	-drive format=raw,file=$(SFSIMG),media=disk,cache=writeback \
	-smp 4 \
	-serial mon:stdio \
	-device isa-debug-exit \
	-nographic
else ifeq ($(arch), riscv32)
qemu_opts := \
	-machine virt \
	-nographic \
	-kernel $(bin)
else ifeq ($(arch), aarch64)
qemu_opts := \
	-machine $(board) \
	-serial null -serial mon:stdio \
	-nographic \
	-kernel $(bin)
endif

ifdef d
qemu_opts := $(qemu_opts) -d $(d)
endif



### build args ###
ifeq ($(arch), riscv32)
ifeq ($(board), fpga)
features := $(features) no_bbl
endif
endif

# Link user binaries at ../user
ifdef link_user
features := $(features) link_user_program
assembly_object_files := $(assembly_object_files) $(user_obj)
endif

features := $(features) board_$(board)
build_args := --target $(target).json --features "$(features)"

ifeq ($(mode), release)
build_args := $(build_args) --release
endif



### prefix ###
ld := $(prefix)-ld
objdump := $(prefix)-objdump
objcopy := $(prefix)-objcopy
cc := $(prefix)-gcc
as := $(prefix)-as
gdb := $(prefix)-gdb

.PHONY: all clean run build asm doc justrun kernel

all: kernel

clean:
	@cargo clean

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

elf-h:
	@$(objdump) -h $(kernel)

$(bin): kernel
ifeq ($(arch), riscv32)
ifeq ($(board), fpga)
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
else ifeq ($(arch), aarch64)
	$(objcopy) $(kernel) --strip-all -O binary $@
endif

kernel:
ifeq ($(arch), x86_64)
	@bootimage build $(build_args)
else
	@cargo xbuild $(build_args)
endif

# make user.o from binary files
$(user_obj): $(user_bins)
	@cd $(user_bin_path) && \
		$(ld) -o $(abspath $@) $(patsubst %, -b binary %, $(notdir $(user_bins)))



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
