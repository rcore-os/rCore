# 中断与异常

## AArch64 异常模型

> 参考：ARMv8 Reference Manual: chapter D1.1, D1.7, D1.10, D1.11, D1.13, D1.14, D1.16.

在 AArch64 中，各种中断被统称为**异常**(exception)，包括：

* Reset.
* Interrupts.
* Memory system aborts.
* Undefined instructions.
* Supervisor calls (SVCs), Secure Monitor calls (SMCs), and hypervisor calls (HVCs).
* Debug exceptions.

这些异常可分为**同步异常**(synchronous exception)与**异步异常**(asynchronous exception)两大类：

* 同步异常：发生在执行一条特定的指令时，包括执行系统调用指令(`svc`、`hvc`)、断点指令(debug exceptions)、Instruction Aborts、Data Aborts 等。
* 异步异常：发生的时机不确定，又被称为**中断**(interrupt)，是发送给处理机的信号，包括 SError、IRQ、FIQ 等。

### 异常处理

当发生异常时，CPU 会根据异常的类型，跳转到特定的地址执行，该地址被称为**异常向量**(exception vector)。

不同类型异常的异常向量通过统一的**向量基地址寄存器**(Vector Base Address Register, VBAR)加上不同的偏移得到。在 EL1、EL2、EL3 下各有一个 VBAR 寄存器 `VBAR_ELx`。此时异常被分为 4 大类，每一类根据异常来源的不同又可分为 4 类，于是共有 16 个异常向量。

4 种类型的异常分别为：

1. Synchronous exception
2. IRQ (Interrupt Request)
3. FIQ (Fast Interrupt Request)
4. SError (System Error)

4 种异常来源分为：

1. Current Exception level with `SP_EL0`. 即发生异常时的异常级别与当前(指跳转到异常向量后)一样，且 `SP = SP_EL0` (`SPsel = 0`)。
2. Current Exception level with `SP_ELx`, `x>0`. 即发生异常时的异常级别与当前一样，且 `SP = SP_ELx` (`SPsel = 1`)。
3. Lower Exception level, where the implemented level immediately lower than the target level is using AArch64. 即发生异常时的异常级别低于当前级别，且运行于 AArch64 模式。
4. Lower Exception level, where the implemented level immediately lower than the target level is using AArch32. 即发生异常时的异常级别低于当前级别，且运行于 AArch32 模式。

16 种异常对应的异常向量相对于 VBAR 的偏移如下表所示：

| Exception taken from | Synchronous | IRQ     | FIQ     | SError  |
|----------------------|-------------|---------|---------|---------|
| CurrentSpEl0         | `0x000`     | `0x080` | `0x100` | `0x180` |
| CurrentSpElx         | `0x200`     | `0x280` | `0x300` | `0x380` |
| LowerAArch64         | `0x400`     | `0x480` | `0x500` | `0x580` |
| LowerAArch32         | `0x600`     | `0x680` | `0x700` | `0x780` |

如果该异常是 Synchronous 或 SError，**异常症状寄存器**(Exception Syndrome Register, ESR)将被设置，用于记录具体的异常类别 EC (exception class) 与 ISS (Instruction Specific Syndrome)。在 EL1、EL2、EL3 下各有一个 ESR 寄存器 `ESR_ELx`。具体的 EC、ISS 编码见官方文档 ARMv8 Reference Manual D1.10.4 节。

### 异常屏蔽

某些异常可以被**屏蔽**(mask)，即发生时不跳转到相应的异常向量。可被屏蔽的异常包括所有异步异常与调试时的同步异常(debug exceptions)，共 4 种，分别由 PSTATE 中 `DAIF` 字段的 4 个位控制：

1. `D`: Debug exception
2. `A`: SError interrupt
3. `I`: IRQ interrupt
4. `F`: FIQ interrupt

### 异常返回

当发生异常时，异常返回地址会被设置，保存在**异常链接寄存器**(Exception Link Register, ELR) `ELR_ELx` 中；当前的**进程状态 PSTATE** 会保存在**保存的进程状态寄存器**(Saved Process Status Register, SPSR) `SPSR_ELx` 中。

异常返回使用 **`eret`** 指令完成。当异常返回时，`pc` 会根据当前特权级被恢复为 `ELR_ELx` 中的，PSTATE 也会被恢复为 `SPSR_ELx` 中的。通过修改 `SPSR_ELx` 中相应的位并进行异常返回，就能使 PSTATE 被修改，从而实现异常级别切换、异常开启/屏蔽等功能。

### 系统调用

一般使用 **`svc`** 指令(supervisor call)完成，将触发一个同步异常。

## RustOS 中的实现

中断与异常部分的代码主要位于模块 [kernel/src/arch/aarch64/interrupt](../../../kernel/src/arch/aarch64/interrupt/) 中。

### 异常启用与屏蔽

在 [interrupt/mod.rs](../../../kernel/src/arch/aarch64/interrupt/mod.rs#L24) 中，通过写入 `DAIFClr` 与 `DAIFSet` 特殊寄存器修改 PSTATE，分别实现了异常的启用与屏蔽(仅针对 IRQ)，代码如下：

```rust
/// Enable the interrupt (only IRQ).
#[inline(always)]
pub unsafe fn enable() {
    asm!("msr daifclr, #2");
}

/// Disable the interrupt (only IRQ).
#[inline(always)]
pub unsafe fn disable() {
    asm!("msr daifset, #2");
}
```

此外，也可在异常返回前修改保存的 `SPSR_EL1` 寄存器，使得异常返回时 PSTATE 改变，从而实现启用或屏蔽异常，详见 [interrupt/context.rs](../../../kernel/src/arch/aarch64/interrupt/context.rs#L26) 中的 `TrapFrame::new_kernel_thread()` 与 `TrapFrame::new_user_thread()` 函数。

### 异常向量

全局符号 `__vectors` 定义了异常向量基址，并在 [interrupt/mod.rs](../../../kernel/src/arch/aarch64/interrupt/mod.rs#L13) 的 `init()` 函数中通过 `msr vbar_el1, x0` 指令，将 `VBAR_EL1` 设为了 `__vectors`。

16 个异常向量分别通过宏 `HANDLER source kind` 定义在 [interrupt/vector.S](../../../kernel/src/arch/aarch64/interrupt/vector.S) 中，代码如下：

```armasm
.macro HANDLER source kind
    .align 7
    stp     lr, x0, [sp, #-16]!
    mov     x0, #\source
    movk    x0, #\kind, lsl #16
    b       __alltraps
.endm
```

不同的异常向量对应的异常处理例程结构相同，仅有 `source` 和 `kind` 不同。`source` 与 `kind` 将会被合并成一个整数并存到寄存器 `x0` 中，以便作为参数传给 Rust 编写的异常处理函数。

由于不同异常向量的间距较少(仅为 `0x80` 字节)，所以不在 `HANDLER` 中做细致的处理，而是统一跳转到 [trap.S](../../../kernel/src/arch/aarch64/interrupt/trap.S#L92) 的 `__alltraps` 中进行处理。

### 异常处理

统一异常处理例程 `__alltraps` 的代码如下：

```armasm
.global __alltraps
__alltraps:
    SAVE_ALL

    # x0 is set in HANDLER
    mrs x1, esr_el1
    mov x2, sp
    bl rust_trap

.global __trapret
__trapret:
    RESTORE_ALL
    eret
```

流程如下：

1. 首先通过宏 `SAVE_ALL` 保存各寄存器，构成 `TrapFrame`。
2. 然后构造函数参数 `x0`、`x1`、`x2`，分别表示异常类型、异常症状 ESR、`TrapFrame`，并调用 Rust 异常处理函数 `rust_trap()`。
3. 当该函数返回时，通过宏 `RESTORE_ALL` 从 `TrapFrame` 中恢复各寄存器。
4. 最后通过 `eret` 指令进行异常返回。

`TrapFrame` 定义在 [interrupt/context.rs](../../../kernel/src/arch/aarch64/interrupt/context.rs#L12)中，结构如下：

```rust
pub struct TrapFrame {
    pub elr: usize,
    pub spsr: usize,
    pub sp: usize,
    pub tpidr: usize, // currently unused
    // pub q0to31: [u128; 32], // disable SIMD/FP registers
    pub x1to29: [usize; 29],
    pub __reserved: usize,
    pub x30: usize, // lr
    pub x0: usize,
}
```

目前保存的寄存器包括：通用寄存器 `x0~x30`、异常返回地址 `elr_el1`、用户栈指针 `sp_el0`、进程状态 `spsr_el1`。由于在 `aarch64-blog_os.json` 中禁用了 NEON 指令，不需要保存 `q0~q31` 这些 SIMD/FP 寄存器。

`rust_trap()` 函数定义在 [interrupt/handler.rs](../../../kernel/src/arch/aarch64/interrupt/handler.rs#L43) 中。首先判断传入的 `kind`：

* 如果是 `Synchronous`：在 [interrupt/syndrome.rs](../../../kernel/src/arch/aarch64/interrupt/syndrome.rs) 中解析 ESR，根据具体的异常类别分别处理断点指令、系统调用、缺页异常等。
* 如果是 IRQ：调用 `handle_irq()` 函数处理 IRQ。
* 其他类型的异常(SError interrupt、Debug exception)暂不做处理，直接调用 `crate::trap::error()`。

#### 系统调用

如果 ESR 的异常类别是 SVC，则说明该异常由系统调用指令 `svc` 触发，紧接着会调用 `handle_syscall()` 函数。

RustOS 的系统调用方式如下(实现在 [user/ucore-ulib/src/syscall.rs](../../../user/ucore-ulib/src/syscall.rs#L47) 中)：

1. 将系统调用号保存在寄存器 `x8`，将 6 参数分别保存在寄存器 `x0~x5` 中。
2. 执行系统调用指令 `svc 0`。
3. 系统调用返回值保存在寄存器 `x0` 中。

在 `handle_syscall()` 函数中，会从 `TrapFrame` 保存的寄存器中恢复系统调用参数，并调用 `crate::syscall::syscall()` 进行具体的系统调用。

#### 缺页异常

缺页异常只会在 MMU 启用后，虚拟地址翻译失败时产生，这时候根据是取指还是访存，分别触发 Instruction Abort 与 Data Abort。此时 ISS 中还记录了具体的状态码，例如：

* Address size fault, level 0~3.
* Translation fault, level 0~3.
* Access flag fault, level 0~3.
* Permission fault, level 0~3.
* Alignment fault.
* TLB conflict abort.
* ...

其中 level 表示在第几级翻译表产生异常。当状态码是 translation fault、access flag fault、permission fault 时，将被判断为是缺页异常，并调用 `handle_page_fault()` 处理缺页异常。

发生 Instruction Abort 与 Data Abort 的虚拟地址将会被保存到 `FAR_ELx` 系统寄存器中。此时再调用 `crate::memory::page_fault_handler(addr)` 来做具体的缺页处理。

#### IRQ

如果该异常是 IRQ，则会调用 [kernel/src/arch/aarch64/board/raspi3/irq.rs](../../../kernel/src/arch/aarch64/board/raspi3/irq.rs#L8) 中的 `handle_irq()` 函数。该函数与具体的硬件板子相关，即使都是在 AArch64 下，不同 board 的 IRQ 处理方式也不一样，所以放到了模块 [kernel/src/arch/aarch64/board/raspi3](../../../kernel/src/arch/aarch64/board/raspi3/) 中，表示是 RPi3 特有的 IRQ 处理方式。

该函数首先会判断是否有时钟中断，如果有就先处理时钟中断：

```rust
let controller = bcm2837::timer::Timer::new();
if controller.is_pending() {
    super::timer::set_next();
    crate::trap::timer();
}
```

其中使用了 crate bcm2837，位于 [crate/bcm2837](../../../crate/bcm2837/) 中，是一个封装良好的访问 RPi3 底层外围设备的库。

然后会遍历所有其他未处理的 IRQ，如果该 IRQ 已注册，就调用它的处理函数：

```rust
for int in Controller::new().pending_interrupts() {
    if let Some(handler) = IRQ_HANDLERS[int] {
        handler();
    }
}
```

IRQ 的注册可通过调用 `register_irq()` 函数进行，实现如下：

```rust
pub fn register_irq(int: Interrupt, handler: fn()) {
    unsafe {
        *(&IRQ_HANDLERS[int as usize] as *const _ as *mut Option<fn()>) = Some(handler);
    }
    Controller::new().enable(int);
}
```
