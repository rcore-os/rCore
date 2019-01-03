# 启动与初始化

## 树莓派启动流程

树莓派的启动流程如下：

1. 第一阶段：第一级 bootloader 位于片上 ROM 中，它挂载 SD 卡中的 FAT32 启动分区，并载入第二级 bootloader。
2. 第二阶段：第二级 bootloader 位于`bootcode.bin` 中，它将载入 GPU 固件代码，并启动 GPU，进入第三级 bootloader。
3. GPU 固件：该阶段将运行 GPU 固件 `start.elf`，它会读取 `config.txt` 中的启动参数，并将内核镜像 `kernel8.img` 复制到 `0x80000` 上。
4. CPU 代码：CPU 从 `0x80000` 处开始执行内核代码。

> 参考：https://github.com/DieterReuter/workshop-raspberrypi-64bit-os/blob/master/part1-bootloader.md

## linker.ld

链接脚本位于 [kernel/src/arch/aarch64/boot/linker.ld](../../../kernel/src/arch/aarch64/boot/linker.ld)，主要内容如下：

```
SECTIONS {
  . = 0x80000; /* Raspbery Pi 3 AArch64 (kernel8.img) load address */

  .boot : {
    KEEP(*(.text.boot)) /* from boot.S */
  }

  . = 0x100000; /* Load the kernel at this address. It's also kernel stack top address */
  bootstacktop = .;

  .text : {
    stext = .;
    *(.text.entry)
    *(.text .text.* .gnu.linkonce.t*)
    . = ALIGN(4K);
    etext = .;
  }

  /* ... */
}
```

几个要点：

* CPU 最先从 `.text.boot (0x80000)` 处开始执行。
* 在 [boot.S](../../../kernel/src/arch/aarch64/boot/boot.S) 中做好了必要的初始化后，将跳转到 `_start (0x100000)`，再从这里跳转到 Rust 代码 `rust_main()`。
* [boot.S](../../../kernel/src/arch/aarch64/boot/boot.S) 的偏移为 `0x80000`，Rust 代码的偏移为 `0x100000`。
* 跳转到 `rust_main()` 后，`0x0~0x100000` 这段内存将被作为内核栈，大小为 1MB，栈顶即 `bootstacktop (0x100000)`。
* [boot.S](../../../kernel/src/arch/aarch64/boot/boot.S) 结束后还未启用 MMU，可直接访问物理地址。

## boot.S

在 RustOS 中，内核将运行在 EL1 上，用户程序将运行在 EL0 上。

CPU 启动代码位于 [kernel/src/arch/aarch64/boot/boot.S](../../../kernel/src/arch/aarch64/boot/boot.S)，负责初始化一些系统寄存器，并将当前异常级别切换到 EL1。

[boot.S](../../../kernel/src/arch/aarch64/boot/boot.S) 的主要流程如下：

1. 获取核的编号，目前只使用 0 号核，其余核将被闲置：

    ```armasm
    .section .text.boot
    boot:
        # read cpu affinity, start core 0, halt rest
        mrs     x1, mpidr_el1
        and     x1, x1, #3
        cbz     x1, setup

    halt:
        # core affinity != 0, halt it
        wfe
        b       halt
    ```

2. 读取当前异常级别：

    ```armasm
    # read the current exception level into x0 (ref: C5.2.1)
    mrs     x0, CurrentEL
    and     x0, x0, #0b1100
    lsr     x0, x0, #2
    ```

3. 如果当前位于 EL3，初始化一些 EL3 下的系统寄存器，并使用 `eret` 指令切换到 EL2：

    ```armasm
    switch_to_el2:
        # switch to EL2 if we are in EL3. otherwise switch to EL1
        cmp     x0, #2
        beq     switch_to_el1

        # set-up SCR_EL3 (bits 0, 4, 5, 7, 8, 10) (A53: 4.3.42)
        mov     x0, #0x5b1
        msr     scr_el3, x0

        # set-up SPSR_EL3 (bits 0, 3, 6, 7, 8, 9) (ref: C5.2.20)
        mov     x0, #0x3c9
        msr     spsr_el3, x0

        # switch
        adr     x0, switch_to_el1
        msr     elr_el3, x0

        eret
    ```

4. 当前位于 EL2，初值化 EL2 下的系统寄存器，并使用 `eret` 指令切换到 EL1：

    ```armasm
    switch_to_el1:
        # switch to EL1 if we are not already in EL1. otherwise continue with start
        cmp     x0, #1
        beq     set_stack

        # set the stack-pointer for EL1
        msr     sp_el1, x1

        # set-up HCR_EL2, enable AArch64 in EL1 (bits 1, 31) (ref: D10.2.45)
        mov     x0, #0x0002
        movk    x0, #0x8000, lsl #16
        msr     hcr_el2, x0

        # do not trap accessing SVE registers (ref: D10.2.30)
        msr     cptr_el2, xzr

        # enable floating point and SVE (SIMD) (bits 20, 21) (ref: D10.2.29)
        mrs     x0, cpacr_el1
        orr     x0, x0, #(0x3 << 20)
        msr     cpacr_el1, x0

        # Set SCTLR to known state (RES1: 11, 20, 22, 23, 28, 29) (ref: D10.2.100)
        mov     x0, #0x0800
        movk    x0, #0x30d0, lsl #16
        msr     sctlr_el1, x0

        # set-up SPSR_EL2 (bits 0, 2, 6, 7, 8, 9) (ref: C5.2.19)
        mov     x0, #0x3c5
        msr     spsr_el2, x0

        # enable CNTP for EL1/EL0 (ref: D7.5.2, D7.5.13)
        # NOTE: This does not actually enable the counter stream.
        mrs     x0, cnthctl_el2
        orr     x0, x0, #3
        msr     cnthctl_el2, x0
        msr     cntvoff_el2, xzr

        # switch
        adr     x0, set_stack
        msr     elr_el2, x0

        eret
    ```

5. 当前位于 EL1，设置栈顶地址为 `_start (0x100000)`，清空 BSS 段的数据：

    ```armasm
    set_stack:
        # set the current stack pointer
        mov     sp, x1

    zero_bss:
        # load the start address and number of bytes in BSS section
        ldr     x1, =sbss
        ldr     x2, =__bss_length

    zero_bss_loop:
        # zero out the BSS section, 64-bits at a time
        cbz     x2, zero_bss_loop_end
        str     xzr, [x1], #8
        sub     x2, x2, #8
        cbnz    x2, zero_bss_loop

    zero_bss_loop_end:
        b       _start
    ```

6. 最后跳转到 Rust 代码 `rust_main()`：

    ```armasm
    .section .text.entry
    .globl _start
    _start:
        # jump to rust_main, which should not return. halt if it does
        bl      rust_main
        b       halt
    ```

## rust_main

在 [boot.S](../../../kernel/src/arch/aarch64/boot/boot.S) 初始化完毕后，会进入 [kernel/src/arch/aarch64/mod.rs](../../../kernel/src/arch/aarch64/mod.rs#L19) 的 Rust 函数 `rust_main()`：

```rust
/// The entry point of kernel
#[no_mangle] // don't mangle the name of this function
pub extern "C" fn rust_main() -> ! {
    memory::init_mmu_early(); // Enable mmu and paging
    board::init_serial_early();

    crate::logging::init();
    interrupt::init();
    memory::init();
    driver::init();
    println!("{}", LOGO);

    crate::process::init();

    crate::kmain();
}
```

流程如下：

1. 建立临时页表，启动 MMU。
2. 初始化串口输入输出，可以使用 `println!()` 等宏了。
3. 初始化 logging 模块，可以使用 `info!()`、`error!()` 等宏了。
4. 初始化中断，其实就是设置了异常向量基址。
5. 初始化内存管理，包括物理页帧分配器与内核堆分配器，最后会建立一个新的页表重新映射内核。
6. 初始化其他设备驱动，包括 Frambuffer、Console、Timer。
7. 初始化进程管理，包括线程调度器、进程管理器，并为每个核建立一个 idle 线程，最后会加载 SFS 文件系统加入用户态 shell 进程。
8. 最后调用 `crate::kmain()`，按调度器轮流执行创建的线程。
