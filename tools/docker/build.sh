wget https://download.qemu.org/qemu-4.1.0.tar.xz
wget https://musl.cc/aarch64-linux-musl-cross.tgz
wget https://more.musl.cc/8/x86_64-linux-musl/riscv32-linux-musl-cross.tgz
wget https://musl.cc/riscv64-linux-musl-cross.tgz
wget https://musl.cc/mipsel-linux-musln32-cross.tgz
wget https://musl.cc/x86_64-linux-musl-cross.tgz
docker build -t rcore .
