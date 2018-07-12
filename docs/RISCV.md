# RISCV 移植记录

## 开发环境

* [riscv-rust/rust](https://github.com/riscv-rust/rust)：使用[官方发布的二进制版本+源码](https://github.com/riscv-rust/rust/releases/tag/riscv-rust-1.26.0-1-dev)
* [riscv-gnu-toolchain](https://github.com/riscv/riscv-gnu-toolchain)：使用OS2018腾讯云中使用的预编译版本

具体配置过程详见[Dockerfile](../riscv-env/Dockerfile)

## Rust-RISCV

### 目标指令集：RISCV32IM

target: riscv32im_unknown_none

由于工具链二进制版本尚未内置此target，因此需提供配置文件：`riscv32-blog_os.json`。

理想情况下，目标指令集应为RISCV32G，即使用全部扩展。但考虑到要把它跑在我们自己实现的CPU上，指令集应该尽量精简，即最好是RISCV32I。此外：

* 为什么用乘除指令扩展？

  Rust核心库中fmt模块会使用乘除运算，若不使用乘除指令，则会依赖LLVM提供的内置函数进行软计算，导致链接错误。这一问题理论上可以通过在xargo中设置依赖compiler-builtin解决。但如此操作后，仍有一个函数`__mulsi3`缺失（32×32）。经查，compiler-builtin中实现了类似的`__muldi3`函数（64×64)，所以理论上可以用它手动实现前者。但如此操作后，还是不对，实验表明`__muldi3`本身也是不正确的。

  总之，没有成功配置不使用M扩展的编译环境，不过日后解决这一问题并不困难。
  
### 原子操作支持

配置文件中与原子操作相关的有两处：

* `feature`中`+a`：使用A指令扩展
* `max-atomic-width`：决定能否使用core中的atomic模块，设为0不可以，设为32可以

二者是否相关，还不能确定。

* 一方面，`riscv-rust/rust`官方配置中，二者是相关的。
* 另一方面，即使不使用A指令扩展，设置`max-atomic-width=32`，也可以编译通过。经检查生成的代码中包含了fence指令。这说明RISCV32I也可以用实现基本同步操作（？）

然而由于LLVM后端对RISCV原子操作支持不完善，无论是否`+a`，当使用Mutex时，它会调用core中的`atomic_compare_exchange`函数，LLVM会发生错误。

鉴于更改上层实现（替换Mutex）工程难度较大，我尝试直接修改core代码，将上述问题函数手动实现。

思路是在关中断环境下，用多条指令完成目标功能。这对于单核环境应该是正确的。

我做了个[补丁](../src/arch/riscv32/atomic.patch)，在进入docker环境后，可运行`make patch-core`应用补丁，确保clean后，再build。

### LLVM Bug

当使用`(u8,u8)`类型时，会触发LLVM Error：

```
llvm::MVT llvm::EVT::getSimpleVT() const: Assertion `isSimple() && "Expected a SimpleValueType!"' failed.
```

在开发过程中，有两个地方遇到了这个错误，一是`log`库，二是`memory::cow`模块。后者修改为`(u16,u16)`就解决了问题，前者在Github上fork了一版，换了一种实现。

## BootLoader

参考[bbl-ucore](https://github.com/ring00/bbl-ucore)及后续的[ucore_os_lab for RISCV32](https://github.com/chyyuu/ucore_os_lab/tree/riscv32-priv-1.10)，使用[bbl](https://github.com/riscv/riscv-pk.git)作为BootLoader。

然而官方版本和bbl-ucore中的fork版本都无法正常编译，使用的是[ucore_os_lab中的修改版本](https://github.com/chyyuu/ucore_os_lab/tree/riscv32-priv-1.10/riscv-pk)。

bbl-ucore使用RISCV1.9的bbl，ucore_os_lab使用RISCV1.10的bbl。后者相比前者，去掉了对内核的内存映射，因此需保证虚实地址一致。

注：事实上ucore_os_lab中的虚实地址并不一致，且没有内存映射，但依然能够运行，应该是由于编译器生成的所有跳转都使用相对偏移。而Rust编译器会生成绝对地址跳转，因此若虚实不一致会导致非法访存。

## Trap

参考资料：

* [bbl-ucore lab1文档](https://ring00.github.io/bbl-ucore/#/lab1)
* [RISCV官方slice](https://riscv.org/wp-content/uploads/2016/07/Tue0900_RISCV-20160712-InterruptsV2.pdf)

### Trap

* 中断帧：32个整数寄存器 + 4个S-Mode状态寄存器
* 开启中断：
  * stvec：设置中断处理函数地址
  * sstatus：SIE bit 开启中断

### Timer

* 开启时钟中断：

  * sie：STIE bit 开启时钟中断
  * sbi::set_timer：设置下次中断时间

* 读取时间：

  * mtime：可读出当前时间（低32bit）

  * mtimeh：当前时间（高32bit），仅RV32有效

    因此RV32下要读取完整时间u64，需循环读取判等，因为指令之间可能被中断，要保证原子性。详见`get_cycle()`。

* 触发中断：

  * mtimecmp(h)：下次触发时钟中断的时间

    当time>=timecmp时，触发中断

    可通过sbi::set_timer设置

## Memory

### 自映射

原x86_64版本使用页表自映射完成修改页表本身的操作。但**RISCV下的页表规范阻碍了自映射的实现**。原因是RISCV页表项中的flags，明确表示它指向的是数据页（VRW），还是下层页表（V）。假如把一个二级页表项，当做一级页表项来解读，就会触发异常。而这是自映射机制中必须的操作。

为了绕开这个问题，就要求**在访问一级页表虚地址期间，将它所对应的二级页表项flags置为VRW**。此外，为了访问二级页表本身，还需要再加一个自映射的二级页表项，其flags为VRW。

制作一个自映射的二级页表过程示意如下：

```rust
fn set_recursive(self: &mut PageTable, recursive_index: usize, frame: Frame) {
    type EF = PageTableFlags;
    self[recursive_index].set(frame.clone(), EF::VALID);
    self[recursive_index + 1].set(frame.clone(), EF::VALID | EF::READABLE | EF::WRITABLE);
}
```

在自映射页表生效后，可用地址：

* (R, R+1, 0) 访问二级页表
* (R, P2, 0) 访问一级页表

*注：地址格式为（P2, P1, Offset），R为自映射下标*

一个值得注意的现象是：在编辑完一级页表、并将其对应的二级页表项flags恢复为V之后，一级页表虚地址还是可以照常访问的，这应该是TLB缓存未失效的缘故。

### 获取内存信息

原x86_64版本使用GRUB进行boot，可通过Multiboot2获取内核段和可用空间的信息。RISCV下bbl无法提供这些信息，只能在linker script中定义各段的起始位置符号，然后在Rust中extern导入。

为了将来适配自己造的CPU（8M RAM），暂时规定内存划分如下：

* 0x80000000 - 0x80020000：BootLoader
* 0x80020000 - 0x80100000：Kernel
* 0x80100000 - 0x80200000：Kernel heap
* 0x80200000 - 0x80800000：对应物理空间用于用户程序

