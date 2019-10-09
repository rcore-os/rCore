FROM rust:latest

# install QEMU
ADD qemu-4.1.0.tar.xz .
RUN cd qemu-4.1.0 \
    && ./configure --target-list=riscv32-softmmu,riscv64-softmmu,mipsel-softmmu,aarch64-softmmu,x86_64-softmmu \
    && make -j
ENV PATH=$PWD/qemu-4.1.0/riscv32-softmmu:$PWD/qemu-4.1.0/riscv64-softmmu:$PWD/qemu-4.1.0/mipsel-softmmu:$PWD/qemu-4.1.0/aarch64-softmmu:$PWD/qemu-4.1.0/x86_64-softmmu:$PWD/qemu-4.1.0:$PATH

# install musl-gcc toolchain
ADD aarch64-linux-musl-cross.tgz .
ADD riscv32-linux-musl-cross.tgz .
ADD riscv64-linux-musl-cross.tgz .
ADD mipsel-linux-musln32-cross.tgz .
ADD x86_64-linux-musl-cross.tgz .
ENV PATH=$PWD/aarch64-linux-musl-cross/bin:$PWD/riscv32-linux-musl-cross/bin:$PWD/riscv64-linux-musl-cross/bin:$PWD/mipsel-linux-musln32-cross/bin:$PWD/x86_64-linux-musl-cross/bin:$PATH

# install others
RUN apt update \
    && apt install less device-tree-compiler -y \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/*

# install Rust tools
RUN cargo install cargo-binutils cargo-xbuild
RUN rustup toolchain add nightly-2019-07-15
RUN rustup component add rust-src llvm-tools-preview --toolchain nightly-2019-07-15
