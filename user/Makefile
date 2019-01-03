# arch = {riscv32, x86_64, aarch64}
# mode = {debug, release}
arch ?= riscv32
mode ?= debug

rust_src_dir := src/bin
rust_bin_path := target/$(arch)-rcore/$(mode)
user_rust_bins := $(patsubst $(rust_src_dir)/%.rs, $(rust_bin_path)/%, $(wildcard $(rust_src_dir)/*.rs))
c_src_dir :=
c_bin_path :=
user_c_bins :=
user_bins := $(user_rust_bins) $(user_c_bins)

build_path := build
sfsroot := $(build_path)/sfsroot-$(arch)
sfsimg := $(build_path)/user-$(arch).img

mksfs := mksfs

build_args := --target targets/$(arch)-rcore.json
ifeq ($(mode), release)
build_args := $(build_args) --release
endif

.PHONY: all clean build-rust build-c build mksfs sfsimg

# not support riscv64 now
ifeq ($(arch), riscv64)
all:
else
all: $(sfsimg)
endif

build-rust:
	@echo Building user rust programs
	@cargo xbuild $(build_args)

build-c:
	@echo Building user C programs

build: build-rust build-c

$(user_rust_bins): build-rust

$(user_c_bins): build-c

mksfs:
ifeq ($(shell which $(mksfs)),)
	@cargo install --git https://github.com/wangrunji0408/SimpleFileSystem-Rust --features="std"
endif

$(sfsroot): $(user_bins)
	@mkdir -p $@
	@cp $^ $@/

$(sfsimg): $(user_bins) $(sfsroot) | mksfs
	@echo Creating SFS image at $@
	@$(mksfs) zip $(sfsroot) $@

sfsimg: $(sfsimg)

clean:
	@cargo clean
	@rm -rf $(build_path)
