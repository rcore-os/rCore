# 代码结构

## 驱动

按照分类放到 `src/drivers` 目录下。

如果是有 device tree 或者 pci 的平台，应当从对应的 bus 中初始化并传递参数。

如果是全局唯一并且代码不会复用的情况，可以写成 singleton。

## ISA 相关代码结构

路径：`src/arch/ISA`。其余的代码尽量不要出现平台相关的代码。

### consts.rs

- KERNEL_OFFSET： 线性映射的偏移
- ARCH：ISA 的名称

### cpu

- fn id()：当前正在运行的 CPU 的 ID

### timer

- fn timer_now()：当前的时间

### interrupt

- fn ack(trap: usize)：确认中断处理
- fn timer()：处理时钟中断

### interrupt/consts

- fn is_page_fault(trap: usize)：是否缺页
- IrqMin：中断的最小 trap
- IrqMax：中断的最大 trap
- Syscall：系统调用的 trap
- Timer：时钟中断的 trap

### syscall

含有所有 syscall 的编号的定义

### board/xxx

针对该架构下某个平台的相关代码

- fn early_init()：早期初始化串口等
- fn early_final()：结束早期初始化串口
- fn init()：初始化
