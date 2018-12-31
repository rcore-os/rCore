# 概述

## Raspberry Pi 简介

本实验的目标是将 Rust OS 移植到 Raspberry Pi 3 Model B+ 上。Raspberry Pi 3B+ 的主要硬件参数如下：

| Raspberry Pi 3B+ | |
|-------|---------|
| 指令集 | ARMv8-A 64 bit |
| 片上系统(SoC) | Broadcom BCM2837B0 |
| 处理器(CPU) | 4 x Cortex-A53 1.4Ghz
| 图形处理器(GPU) | Broadcom VideoCore IV |
| 内存 | 1GB (与 GPU 共享) |

## AArch64 简介

## 官方文档

* [ARM Architecture Reference Manual ARMv8, for ARMv8-A architecture profile](https://static.docs.arm.com/ddi0487/da/DDI0487D_a_armv8_arm.pdf)：AArch64 的完整文档，有 7000 多页，最为详细。
* [ARM Cortex-A Series Programmer’s Guide for ARMv8-A](http://infocenter.arm.com/help/topic/com.arm.doc.den0024a/DEN0024A_v8_architecture_PG.pdf)：可认为是上一文档的精简版，仅有不到 300 页。
* [BCM2837 ARM Peripherals](https://web.stanford.edu/class/cs140e/docs/BCM2837-ARM-Peripherals.pdf)：Raspberry Pi SoC BCM283x 系列的外围设备文档，包含对 GPIO、中断控制器、mini UART、System Timer 等外围设备的访问。
* [BCM2836 ARM-local peripherals](https://www.raspberrypi.org/documentation/hardware/raspberrypi/bcm2836/QA7_rev3.4.pdf)：仅用于如何使用 ARM Generic Timer。
* [Raspberry Pi firmware](https://github.com/raspberrypi/firmware)：Raspberry Pi 二进制固件，部分开源，其中最有价值的是 [mailbox](https://github.com/raspberrypi/firmware/wiki) 的文档。

## 其他参考

* [Stanford CS140e](http://cs140e.stanford.edu/)：Stanford CS140e 课程，一个用 Rust 语言编写的 Raspberry Pi 3 操作系统，包含串口输入输出、文件系统、进程管理等功能，但没有虚拟内存管理。

* [Learning operating system development using Linux kernel and Raspberry Pi](https://github.com/s-matyukevich/raspberry-pi-os)：一个用 C 语言编写的 Raspberry Pi 3 操作系统，仿照 Linux，特点是文档非常详细。其中 Kernel Initialization、Interrupt handling、Virtual memory management 部分很有参考价值。

* [Bare Metal Rust Programming on Raspberry Pi 3](https://github.com/bztsrc/raspi3-tutorial)：另一个用 C 语言编写的 Raspberry Pi 3 操作系统。

* [Bare Metal Rust Programming on Raspberry Pi 3 (Rust)](https://github.com/rust-embedded/rust-raspi3-tutorial)：上一个项目的 Rust 版本，主要参考的是虚拟内存部分。
