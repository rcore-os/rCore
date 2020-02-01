# rCore

[![CI](https://github.com/rcore-os/rCore/workflows/CI/badge.svg?branch=master)](https://github.com/rcore-os/rCore/actions)

Rust version of THU [uCore OS Plus](https://github.com/chyyuu/ucore_os_plus).

Going to be the next generation teaching operating system.

Supported architectures and boards:

* x86_64: PC (i5/i7)
* RISCV32/64: HiFive Unleashed, Kendryte K210, [FPGA running Rocket Chip](https://github.com/jiegec/fpga-zynq)
* AArch64: Raspberry Pi 3B+
* MIPS32: [TrivialMIPS](https://github.com/Harry-Chen/TrivialMIPS)

![demo](./docs/2_OSLab/os2atc/demo.png)

## Building

### Environment

* [Rust](https://www.rust-lang.org) toolchain
* [QEMU](https://www.qemu.org) >= 4.1.0
* [musl-based GCC toolchains](https://musl.cc/) (only for building [user programs](https://github.com/rcore-os/rcore-user))

Setup on Linux or macOS:

```bash
$ rustup component add rust-src llvm-tools-preview
```

Or use Docker container:

```bash
$ docker run -it -v $PWD:$PWD -w $PWD wangrunji0408/rcore
```

### How to run

```bash
$ git clone https://github.com/rcore-os/rCore.git --recursive
$ cd rCore/user
$ make sfsimg prebuilt=1 arch=x86_64
$ cd ../kernel
$ make run ARCH=x86_64 LOG=info
```

See [Makefile](kernel/Makefile) for more usages.

## Maintainers

| Module | Maintainer            |
|--------|-----------------------|
| x86_64 | @wangrunji0408        |
| RISC-V  | @jiegec               |
| ARM (Raspi3) | @equation314    |
| MIPS   | @Harry_Chen @miskcoo   |
| Memory, Process, File System | @wangrunji0408          |
| Network with drivers | @jiegec |
| GUI    | @equation314          |

## History

This is a project of THU courses:

* [Operating System (2018 Spring) ](http://os.cs.tsinghua.edu.cn/oscourse/OS2018spring/projects/g11)
* [Comprehensive Experiment of Computer System (2018 Summer)](http://os.cs.tsinghua.edu.cn/oscourse/csproject2018/group05)
* [Operating System Train (2018 Autumn)](http://os.cs.tsinghua.edu.cn/oscourse/OsTrain2018)
* [Operating System (2019 Spring)](http://os.cs.tsinghua.edu.cn/oscourse/OS2019spring/projects)
* [Operating System Train (2019 Autumn)](http://os.cs.tsinghua.edu.cn/oscourse/OsTrain2019)

[Reports](./docs) and [Dev docs](https://rucore.gitbook.io/rust-os-docs/) (in Chinese)

It's based on [BlogOS](https://github.com/phil-opp/blog_os) , a demo project in the excellent tutorial [Writing an OS in Rust (First Edition)](https://os.phil-opp.com/first-edition/).

## License

The source code is dual-licensed under MIT or the Apache License (Version 2.0).
