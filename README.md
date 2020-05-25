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

## libc-test 

### 安装

在 `user` 目录下执行

```bash
$ git clone git://repo.or.cz/libc-test
```

### 编译

考虑到在 rCore 中编译所有测例耗费时间过长，所以选择在本机用 musl-gcc 编译。
在本机执行

```bash
$ make
$ rm src/*/*.err
```

随后修改 `user` 目录下的 `Makefile` 文件，将 `libc-test\`打包进入文件系统。

### 在 rCore 中测试

进入 `libc\` 目录，执行脚本

```bash
$ ash runtest.sh
```

在测试测例前控制台会先打印当前测例名。若测试成功则顺次测试下一个测例，若失败则会打印额外信息，当遇到更严重的错误时可能导致 rCore 卡死或崩溃。例如在测试 `math` 库中的 `sqrt` 时，若测试失败，则输出为

```
run sqrt
sqrt failed
```

在结束后，可前往对应测例所在目录下，通过查看测例所对应的 `.err` 文件查看失败的原因。

当遇到使得 rCore 崩溃的测例时，手动记录当前测例在 `runtest.sh` 中的位置，手动更新 `user\` 中的文件，使其从下一个测例开始测试，并记录中间若干测例的测试结果。如此反复，直到测试过所有测例。

### 目前测试的结果

- [ ] 尚未通过的测例 (215/473)
    - [ ] `pthread` 相关：可能由于缺少相关信号
    - [ ] `math` 相关：由于缺少对 `mxcsr` 寄存器的支持，导致获取 `FP Exceptions` 失败，从而无法通过相关测例中对 `FP Exceptions` 的校验。极少数情况出现对于 bad cases 的计算错误。
    - [ ] `sync` 相关

具体的测试结果可参考 `user/libc-test/` 目录下的三个 `RECORD.txt` 文件。