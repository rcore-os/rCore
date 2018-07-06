# RISCV 移植记录

## 开发环境

* [riscv-rust/rust](https://github.com/riscv-rust/rust)：使用[官方发布的二进制版本+源码](https://github.com/riscv-rust/rust/releases/tag/riscv-rust-1.26.0-1-dev)
* [riscv-gnu-toolchain](https://github.com/riscv/riscv-gnu-toolchain)：使用OS2018腾讯云中使用的预编译版本

具体配置过程详见[Dockerfile](../riscv-env/Dockerfile)

## Rust-RISCV

### 目标指令集：RISCV32IMA

target: riscv32ima_unknown_none

由于工具链二进制版本尚未内置此target，因此需提供配置文件：`riscv32-blog_os.json`。

理想情况下，目标指令集应为RISCV32G，即使用全部扩展。但考虑到要把它跑在我们自己实现的CPU上，指令集应该尽量精简，即最好是RISCV32I。此外：

* 为什么用原子指令扩展？

  RustOS依赖的库中，大部分都使用了Rust核心库的原子操作（core::sync::atomic）。

  如果目标指令集不支持原子操作，会导致无法编译。

  然而LLVM后端尚不完全支持原子指令扩展，因此这条路可能走不通，需要魔改Rust标准库。

* 为什么用乘除指令扩展？

  Rust核心库中fmt模块会使用乘除运算，若不使用乘除指令，则会依赖LLVM提供的内置函数进行软计算，导致链接错误。这一问题理论上可以通过在xargo中设置依赖compiler-builtin解决。但如此操作后，仍有一个函数`__mulsi3`缺失（32×32）。经查，compiler-builtin中实现了类似的`__muldi3`函数（64×64)，所以理论上可以用它手动实现前者。但如此操作后，还是不对，实验表明`__muldi3`本身也是不正确的。

  总之，没有成功配置不使用M扩展的编译环境，不过日后解决这一问题并不困难。

## BootLoader

参考[bbl-ucore](https://github.com/ring00/bbl-ucore)及后续的[ucore_os_lab for RISCV32](https://github.com/chyyuu/ucore_os_lab/tree/riscv32-priv-1.10)，使用[bbl](https://github.com/riscv/riscv-pk.git)作为BootLoader。

然而官方版本和bbl-ucore中的fork版本都无法正常编译，使用的是[ucore_os_lab中的修改版本](https://github.com/chyyuu/ucore_os_lab/tree/riscv32-priv-1.10/riscv-pk)。

bbl-ucore使用RISCV1.9的bbl，ucore_os_lab使用RISCV1.10的bbl。后者相比前者，去掉了对内核的内存映射，因此需保证虚实地址一致。

注：事实上ucore_os_lab中的虚实地址并不一致，且没有内存映射，但依然能够运行，应该是由于编译器生成的所有跳转都使用相对偏移。而Rust编译器会生成绝对地址跳转，因此若虚实不一致会导致非法访存。



