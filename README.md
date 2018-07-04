# RustOS for x86_64 SMP

## Port to RISCV (WIP)

2018年计算机系统综合实验

[Project Wiki](http://os.cs.tsinghua.edu.cn/oscourse/csproject2018/group05)

[Documents](./docs/RISCV.md)

## Summary

[![Build Status](https://travis-ci.org/wangrunji0408/RustOS.svg?branch=master)](https://travis-ci.org/wangrunji0408/RustOS)

A project of THU OS2018 spring.

[Project Wiki](http://os.cs.tsinghua.edu.cn/oscourse/OS2018spring/projects/g11)

[MidReport](./docs/MidReport.md)

[FinalReport](./docs/FinalReport.md)

The goal is to write a mini OS in Rust with multicore supporting.

It will start from the post of the [Writing an OS in Rust](http://os.phil-opp.com) series. Then reimplement [xv6-x86_64](https://github.com/jserv/xv6-x86_64) in Rust style.

## Building

You need to have `nasm`, `grub-mkrescue`, `xorriso`, `qemu`, a nightly Rust compiler, and `xargo` installed. Then you can run it using `make run`.

A docker image is available and recommanded. Read [this](docker/README.md) for details.

## License

The source code is dual-licensed under MIT or the Apache License (Version 2.0).
