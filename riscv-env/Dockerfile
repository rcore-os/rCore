FROM ubuntu:latest

WORKDIR /rust

# Rust toolchain bins
# https://github.com/riscv-rust/rust/releases/download/riscv-rust-1.26.0-1-dev/rust-1.26.0-dev-x86_64-unknown-linux-gnu.tar.xz
ADD rust-1.26.0-dev-x86_64-unknown-linux-gnu.tar.xz .

# Rust src
# https://github.com/riscv-rust/rust/archive/riscv-rust-1.26.0-1-dev.zip
# with submodule: libcompiler_builtins, stdsimd
ADD rust-riscv-rust-1.26.0-1-dev.tar.gz .

# RISCV32 toolchain
# From tencent cloud for OS2018: ssh 2015011279@140.143.187.14
ADD rv32-toolchains-prebuild.tar.bz2 .

# Dependencies
RUN apt-get update && apt-get install -q -y --no-install-recommends libssl1.0.0 curl libssh2-1 gcc make git libc6-dev
# Rust bins need this
RUN mkdir -p /gnu/store/n6acaivs0jwiwpidjr551dhdni5kgpcr-glibc-2.26.105-g0890d5379c/lib \
	&& ln -s /lib/x86_64-linux-gnu/ld-linux-x86-64.so.2 /gnu/store/n6acaivs0jwiwpidjr551dhdni5kgpcr-glibc-2.26.105-g0890d5379c/lib/ld-linux-x86-64.so.2
# Install Rust toolchains
RUN ./rust-1.26.0-dev-x86_64-unknown-linux-gnu/install.sh && rm -rf ./rust-1.26.0-dev-x86_64-unknown-linux-gnu
# Install xargo
RUN cargo install xargo
# Env
ENV PATH=~/.cargo/bin:/rust/install-rv32/bin:$PATH
ENV XARGO_RUST_SRC=/rust/rust-riscv-rust-1.26.0-1-dev/src
RUN ln -s ~/.cargo/bin/xargo /usr/local/bin/xargo