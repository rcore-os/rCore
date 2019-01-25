# 上下文切换

平台无关的代码位于 [kernel/src/process/context.rs](../../../kernel/src/process/structs.rs) 中，而平台相关(aarch64)的代码位于 [kernel/src/arch/aarch64/interrupt/context.rs](../../../kernel/src/arch/aarch64/interrupt/context.rs) 中。

## 相关数据结构

在 [kernel/src/arch/aarch64/interrupt/context.rs](../../../kernel/src/arch/aarch64/interrupt/context.rs) 中定义了下列数据结构：

1. `TrapFrame`:

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

    在陷入异常时向栈中压入的内容，由 [trap.S](../../../kernel/src/arch/aarch64/interrupt/trap.S#L92) 的 `__alltraps` 构建。详见“中断与异常”相关章节。

2. `ContextData`:

    ```rust
    struct ContextData {
        x19to29: [usize; 11],
        lr: usize,
    }
    ```

    执行上下文切换时向栈中压入的内容，由 `__switch()` 函数构建。仅需保存 callee-saved 寄存器(被调用者保存，即 `x19~x30`)。详见下节“切换流程”。

3. `InitStack`:

    ```rust
    pub struct InitStack {
        context: ContextData,
        tf: TrapFrame,
    }
    ```

    对于新创建的线程，不仅要向栈中压入 `ContextData` 结构，还需手动构造 `TrapFrame` 结构。为了方便管理就定义了 `InitStack` 包含这两个结构体。

4. `Context`:

    ```rust
    pub struct Context {
        stack_top: usize,
        ttbr: PhysFrame,
        asid: Asid,
    }
    ```

    每个进程控制块 `Process` ([kernel/src/process/context.rs](../../../kernel/src/process/structs.rs#L13)) 都会维护一个平台相关的 `Context` 对象，在 AArch64 中包含下列信息：

    1. `stack_top`：内核栈顶地址
    2. `ttbr`：页表基址
    3. `asid`：Address Space ID，详见下文“页表切换与 ASID 机制”

## 切换流程

在 [kernel/src/process/context.rs](../../../kernel/src/process/structs.rs#L22) 里，`switch_to()` 是平台无关的切换函数，最终会调用 [kernel/src/arch/aarch64/interrupt/context.rs](../../../kernel/src/arch/aarch64/interrupt/context.rs#L129) 里平台相关的切换函数 `Context::switch()`：

```rust
pub unsafe fn switch(&mut self, target: &mut Self) {
    target.asid = ASID_ALLOCATOR.lock().alloc(target.asid);

    // with ASID we needn't flush TLB frequently
    ttbr_el1_write_asid(1, target.asid.value, target.ttbr);
    barrier::dsb(barrier::ISH);
    Self::__switch(&mut self.stack_top, &mut target.stack_top);
}
```

### 页表切换与 ASID 机制

首先进行的是页表的切换，即向 `TTBR1_EL1` 寄存器写入目标线程页表基址 `target.ttbr`。一般来说，切换页表后需要刷新 TLB，不过 ARMv8 引入了 ASID (Address Space ID) 机制来避免频繁刷新 TLB。

#### ASID 机制

在页表项描述符中，有一个 nG 位，如果该位为 0，表示这页内存是全局可访问的(用于内核空间)；如果该位为 1，表示这页内存不是全局可访问的，只有特定线程可访问。具体地，如果页表项中该位为 1，当访问相应虚拟地址更新 TLB 时，会有额外的信息被写入 TLB，该信息即 ASID，由操作系统分配，下次在 TLB 中查找该虚拟地址时就会检查 TLB 表项中的 ASID 是否与当前 ASID 匹配。相当于为不同的 ASID 各自创建了一个页表。

ASID 的大小可以为 8 位或 16 位，由 `TCR_EL1` 的 AS 字段指定，当前的 ASID 保存在 TTBR 的高位中，也可以由 `TCR_EL1` 的 `A1` 字段指定是 `TTBR0_EL1` 还是 `TTBR1_EL1`。在 RustOS 中，ASID 大小为 16 位，当前 ASID 保存在 `TTBR1_EL1` 的高 16 位。

在 `switch()` 函数里，首先会为目标线程分配一个 ASID，然后同时将该 ASID 与 `target.ttbr` 写入 `TTBR1_EL1` 即可，无需进行 TLB 刷新。

#### ASID 的分配

ASID 的分配需要保证同一时刻不同线程的 ASID 是不同的。这一部分参考了 Linux，主要思想是每次上下文切换时检查该线程原来的 ASID 是否有效，如果无效需要重新分配并刷新 TLB。

使用的数据结构如下：

```rust
struct Asid {
    value: u16,
    generation: u16,
}

struct AsidAllocator(Asid);
```

一个 ASID 结构由 16 位的 `value` 和 `generation` 组成，`value` 即 ASID 的具体值，`generation` 相当于时间戳。初始的 ASID 两个值都是 0，一定是无效的。该结构也被用于实现 ASID 分配器 `AsidAllocator`，此时该结构表示上一个被分配出去的 ASID。

```rust
const ASID_MASK: u16 = 0xffff;

impl AsidAllocator {
    fn new() -> Self {
        AsidAllocator(Asid { value: 0, generation: 1 })
    }

    fn alloc(&mut self, old_asid: Asid) -> Asid {
        if self.0.generation == old_asid.generation {
            return old_asid;
        }

        if self.0.value == ASID_MASK {
            self.0.value = 0;
            self.0.generation = self.0.generation.wrapping_add(1);
            if self.0.generation == 0 {
                self.0.generation += 1;
            }
            tlb_invalidate_all();
        }
        self.0.value += 1;
        return self.0;
    }
}
```

分配的流程如下：

1. 判断 `old_asid` 是否等于 `self.0.generation`，如果相等说明这一代的 ASID 还是有效的，直接返回 `old_asid`。
2. 否则，`old_asid` 已失效，如果当前代的 65535 个 ASID 没有分配完，就直接分配下一个。
3. 如果当前代的 65535 个 ASID 都分配完了，就开始新的一代，同时刷新 TLB。

### 寄存器与栈的切换

这一部分即 `Context` 的 `__switch()` 函数，传入的两个参数 `_self_stack` 与 `_target_stack` 是两个引用，分别用于保存**当前线程内核栈顶**与**目标线程内核栈顶**。

该函数用汇编实现(两个参数分别保存在 `x0` 和 `x1` 寄存器中)：

```armasm
mov x10, #-(12 * 8)
add x8, sp, x10
str x8, [x0]
stp x19, x20, [x8], #16     // store callee-saved registers
stp x21, x22, [x8], #16
stp x23, x24, [x8], #16
stp x25, x26, [x8], #16
stp x27, x28, [x8], #16
stp x29, lr, [x8], #16

ldr x8, [x1]
ldp x19, x20, [x8], #16     // restore callee-saved registers
ldp x21, x22, [x8], #16
ldp x23, x24, [x8], #16
ldp x25, x26, [x8], #16
ldp x27, x28, [x8], #16
ldp x29, lr, [x8], #16
mov sp, x8

str xzr, [x1]
ret
```

流程如下：

1. 保存**当前栈顶** `sp` 到 `_self_stack` (`x0`)，保存 **callee-saved 寄存器**到当前栈上。
2. 从 `_target_stack` (`x1`) 获取目标线程的**内核栈顶**，从目标线程内核栈顶恢复 **callee-saved 寄存器**。
3. 将 `sp` 设为目标线程内核栈顶，将 `_target_stack` (`x1`) 里的内容清空。
4. 使用 `ret` 指令返回，这会跳转到目标线程 `lr` 寄存器中存放的地址。

为什么只保存了 `sp` 与 callee-saved 寄存器，而不是所有寄存器？因为执行上下文切换就是在调用一个函数，在调用前后编译器会自动保存并恢复 caller-saved 寄存器(调用者保存，即 `x0~x18`)。

### 异常级别切换

异常发生前的异常级别保存在 `TrapFrame` 中 `spsr` 的相应位，在异常返回后会恢复给 PSTATE，实现异常级别切换。通过构造特定的 `spsr` 可让新线程运行在指定的异常级别。

## 创建新线程

线程可通过下列三种方式创建：

1. 创建新的**内核线程**：直接给出一个内核函数。
2. 创建新的**用户线程**：解析 ELF 文件。
3. 从一个线程 **fork** 出一个新线程：通过 `fork` 系统调用。

三种线程的平台无关创建流程实现在 [kernel/src/process/context.rs](../../../kernel/src/process/structs.rs#L40) 里，最终会分别调用 [kernel/src/arch/aarch64/interrupt/context.rs](../../../kernel/src/arch/aarch64/interrupt/context.rs#L146) 里的 `new_kernel_thread()`、`new_user_thread()` 和 `new_fork()` 这三个函数创建平台相关的 `Context` 结构。

在这三个函数里，会构造 `ContextData` 与 `TrapFrame` 结构，构成一个 `InitStack`，并向新线程的内核栈压入 `InitStack` 结构，最后将新内核栈顶地址、页表基址等信息构成 `Context` 结构返回。这两个结构的构造方式如下：

* `ContextData`:

    三种线程的初始 `ContextData` 结构都一样：清空 `x19~x29` 寄存器，将 `lr` 寄存器设为 `__trapret`，表示在 `__switch()` 结束后立即返回 `__trapret`，避免破坏构建好的栈帧结构。

* `TrapFrame`:

    三种线程的 `TrapFrame` 各不相同：

    1. 内核线程：

        | `TrapFrame` 中的字段| 值                                     |
        |---------------------|----------------------------------------|
        | `x0`                | 内核线程参数 `arg`                     |
        | `sp`                | 内核栈顶地址 `kstack_top`              |
        | `elr`               | 内核线程入口函数 `entry` 的地址        |
        | `spsr`              | `0b1101_00_0101`，切换到 EL1，启用 IRQ |
        | 其他                | 清零                                   |

    2. 用户线程：

        | `TrapFrame` 中的字段| 值                                     |
        |---------------------|----------------------------------------|
        | `sp`                | 用户栈顶地址 `ustack_top`              |
        | `elr`               | 用户线程入口地址 `entry_addr`          |
        | `spsr`              | `0b1101_00_0000`，切换到 EL0，启用 IRQ |
        | 其他                | 清零                                   |

        注意用户线程是根据 ELF 文件创建的，参数即命令行参数，通过栈而不是寄存器传递。

    3. fork 线程：直接复制父线程的 `TrapFrame`，并将 fork 的返回值 `x0` 设为 0。
