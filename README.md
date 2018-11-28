# RustOS

[![Build Status](https://travis-ci.org/wangrunji0408/RustOS.svg?branch=master)](https://travis-ci.org/wangrunji0408/RustOS)

Rust version of THU [uCore OS](https://github.com/chyyuu/ucore_os_lab/).

Going to be the next generation teaching operating system.

Support arch: x86_64, RISCV32I, AArch64 (WIP).

[Dev docs](https://rucore.gitbook.io/rust-os-docs/) (in Chinese)

## Summary

This is a project of THU courses:

* Operating System (2018 Spring) 
* Comprehensive Experiment of Computer System (2018 Summer)
* Operating System Train (2018 Autumn)

Project wiki (internal access only): [OS](http://os.cs.tsinghua.edu.cn/oscourse/OS2018spring/projects/g11), [CECS](http://os.cs.tsinghua.edu.cn/oscourse/csproject2018/group05), [OST](http://os.cs.tsinghua.edu.cn/oscourse/OsTrain2018)

Reports (in Chinese): [docs](./docs)



The initial goal is to write a mini OS in Rust with multi-core support. More specifically, it would start from the post of the [Writing an OS in Rust](http://os.phil-opp.com) series, then reimplement [xv6-x86_64](https://github.com/jserv/xv6-x86_64) in Rust style.

In fact, it's more complicated than we expected to write an OS starting from scratch. So by the end of OS course, we only finished rewriting [ucore_os_lab](https://github.com/chyyuu/ucore_os_lab), without multi-core support. Then as a part of [CECS project](https://github.com/riscv-and-rust-and-decaf), we ported it from x86_64 to RISCV32I, and made it work on our FPGA CPU.

## Building

### Environment

* [Rust](https://www.rust-lang.org) toolchain at nightly
* Cargo tools: [cargo-xbuild](https://github.com/rust-osdev/cargo-xbuild), [bootimage](https://github.com/rust-osdev/bootimage)
* QEMU >= 2.12.0
* [RISCV64 GNU toolchain](https://www.sifive.com/products/tools/) (for riscv32)
* [AArch64 GNU toolchain](https://web.stanford.edu/class/cs140e/assignments/0-blinky/) (for aarch64)

### How to run

```bash
rustup component add rust-src
cargo install cargo-xbuild bootimage
```

```bash
git clone https://github.com/wangrunji0408/RustOS.git --recursive
cd RustOS/kernel
rustup override set nightly
make run arch=riscv32|x86_64|aarch64
```

## License

The source code is dual-licensed under MIT or the Apache License (Version 2.0).
