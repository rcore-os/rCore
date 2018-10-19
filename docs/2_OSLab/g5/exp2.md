# 2018操作系统专题训练

# 实验2：方案设计文档

计53 王润基 2015011279

## 实验目标

**基于RustOS，参考sv6完成多核实现和优化。**

分为以下三个子任务：

1. 实现x86_64和RISCV下的多核启动和通信
2. 拓展线程管理模块，使之支持多核调度
3. 学习sv6进行多核优化

## 相关工作和实验方案

1. 实现x86_64和RISCV下的多核启动和通信

   x86_64下的多核启动已经完成，下面计划将其移植到Rust-OSDev项目的Bootloader中。

   RISCV下尚未实现，这块第3组同学们有丰富经验。

   这部分计划与第3组合作，在第4周内完成。

2. 拓展线程管理模块，使之支持多核调度

   参照xv6 / ucore SMP实现一个可工作的版本。

   计划在第5周内完成。

3. 学习sv6进行多核优化

   已经完成[sv6 for RV64](https://github.com/twd2/sv6)在macOS上的复现。

   正在研究代码，并准备日后与twd2交流。

   计划在第6周移植一两个简单的实现到RustOS，并在之后视时间精力将其它部分逐渐移植过来。



   参考论文：

   * [The Scalable Commutativity Rule: Designing Scalable Software for Multicore Processors](https://pdos.csail.mit.edu/papers/commutativity:sosp13.pdf)：Commuter项目论文，如何定量测试OS的并行度。鉴于时间有限，将其应用到RustOS应该无法在本学期完成。
   * [RadixVM: Scalable Address Spaces for Multithreaded Applications](http://pdos.csail.mit.edu/papers/radixvm:eurosys13.pdf)：内存管理相关
   * [Scaling a file system to many cores using an operation log](http://delivery.acm.org/10.1145/3140000/3132779/p69-bhat.pdf?ip=183.172.124.170&id=3132779&acc=OA&key=BF85BBA5741FDC6E%2E587F3204F5B62A59%2E4D4702B0C3E38B35%2EEE2C838055815368&__acm__=1539103199_b0979df5a4432f0766f604f7a6e4809b)：文件系统相关

