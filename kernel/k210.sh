#!/bin/bash

# cargo install cargo-xbuild --path /path/to/cargo-xbuild-0.5.6/

# QEMU
# make run arch=riscv64

# k210 riscv64
# 生成的镜像太大了，还需要进一步分析裁剪 
make install arch=riscv64 board=k210 mode=debug

