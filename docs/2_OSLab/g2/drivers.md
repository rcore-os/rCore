# 设备驱动

树莓派上有着丰富的外围设备(peripherals)，物理地址空间 `0x3F000000~0x3FFFFFFF` 专门用于访问外围设备。

一个设备一般提供多个可供访问的 IO 地址，一般 4 字节对齐。将它们按给定的偏移构造结构体，并使用 crate [volatile](https://crates.io/crates/volatile) 抽象为一些寄存器，可方便地对这些 IO 地址进行读写，例如：

```rust
const INT_BASE: usize = IO_BASE + 0xB000 + 0x200;

#[repr(C)]
#[allow(non_snake_case)]
struct Registers {
    IRQBasicPending: ReadOnly<u32>,
    IRQPending: [ReadOnly<u32>; 2],
    FIQControl: Volatile<u32>,
    EnableIRQ: [Volatile<u32>; 2],
    EnableBasicIRQ: Volatile<u32>,
    DisableIRQ: [Volatile<u32>; 2],
    DisableBasicIRQ: Volatile<u32>,
}

pub fn new() -> Controller {
    Controller {
        registers: unsafe { &mut *(INT_BASE as *mut Registers) },
    }
}
```

这些外围设备的最底层驱动实现在 crate [bcm2837](../../../crate/bcm2837/) 中，包括：

* GPIO
* Mini UART
* Mailbox
* Timer

一些稍微高级的与具体硬件板子相关的驱动实现在 [kernel/src/arch/aarch64/board/raspi3](../../../kernel/src/arch/aarch64/board/raspi3/) 中，包括：

* Framebuffer
* IRQ
* Mailbox property interface
* Serial

更高级的硬件无关的驱动实现在 [kernel/src/arch/aarch64/driver](../../../kernel/src/arch/aarch64/driver/) 中，包括：

* Console

## GPIO

> 参考：BCM2837 ARM Peripherals: chapter 6, General Purpose I/O (GPIO).

目前 RustOS 中的 GPIO 驱动只是为了初始化 mini UART 而使用，实现在 crate [bcm2837](../../../crate/bcm2837/) 的 [gpio.rs](../../../crate/bcm2837/src/gpio.rs) 中。主要提供两个功能：

* 设置引脚模式
* 设置引脚上拉/下拉状态

### 设置引脚模式

引脚模式有 8 种：输入、输出与 alternative function 0~5。根据引脚编号向相应的 `FSEL` 寄存器的相应位写入模式代码即可。

```rust
pub fn into_alt(self, function: Function) -> Gpio<Alt> {
    let select = (self.pin / 10) as usize;
    let offset = 3 * (self.pin % 10) as usize;
    self.registers.FSEL[select].update(|value| {
        *value &= !(0b111 << offset);
        *value |= (function as u32) << offset;
    });
    self.transition()
}

pub fn into_output(self) -> Gpio<Output> {
    self.into_alt(Function::Output).transition()
}

pub fn into_input(self) -> Gpio<Input> {
    self.into_alt(Function::Input).transition()
}
```

### 设置引脚上拉/下拉状态

引脚的上拉/下拉状态有 3 种：上拉(`10`)、下拉(`01`)与不拉(`00`)。设置该状态的流程如下：

1. 向 `PUD` 寄存器写入状态代码；
2. 等待 150 个时钟周期；
3. 根据引脚编号向相应的 `PUDCLK` 寄存器的相应位写入 1；
4. 等待 150 个时钟周期；
5. 向 `PUD` 寄存器写入 0；
6. 根据引脚编号向相应的 `PUDCLK` 寄存器的相应位写入 0。

```rust
pub fn set_gpio_pd(&mut self, pud_value: u8) {
    let index = if self.pin >= 32 { 1 } else { 0 };

    self.registers.PUD.write(pud_value as u32);
    delay(150);
    self.registers.PUDCLK[index as usize].write((1 << self.pin) as u32);
    delay(150);
    self.registers.PUD.write(0);
    self.registers.PUDCLK[index as usize].write(0);
}
```

## Mini UART

> 参考：BCM2837 ARM Peripherals: chapter 2, Auxiliaries: UART1 & SPI1, SPI2; chapter 6, General Purpose I/O (GPIO), page 101~102.

Mini UART 可用于树莓派与上位机直接的通信，一般被称为“串口”。该驱动实现简单，在没有显示器、键盘等驱动时是一种非常好的输入输出设备。

RustOS 中 mini UART 的驱动主要实现在 crate [bcm2837](../../../crate/bcm2837/) 的 [mini_uart.rs](../../../crate/bcm2837/src/mini_uart.rs) 中。在 [kernel/src/arch/aarch64/board/raspi3/serial.rs](../../../kernel/src/arch/aarch64/board/raspi3/serial.rs) 中将其封装为了一个 `SerialPort`，以便通过统一的接口调用。

### 初始化

初始化 mini UART 的流程如下：

1. 向 `AUX_ENABLES` 寄存写 1，启用 mini UART；
2. 将 GPIO 的 14/15 引脚都设为 alternative function ALT5 (TXD1/RXD1) 模式，并都设为不拉状态；
3. 配置 mini UART 参数：

    1. 暂时禁用接收器与发送器；
    2. 启用接收中断，禁用发送中断；
    3. 设置数据大小为 8 bit；
    4. 设置 RTS line 为 high；
    5. 设置波特率为 115200；
    6. 重新启用接收器与发送器。

```rust
pub fn init(&mut self) {
    // Enable the mini UART as an auxiliary device.
    unsafe { (*AUX_ENABLES).write(1) }

    Gpio::new(14).into_alt(Function::Alt5).set_gpio_pd(0);
    Gpio::new(15).into_alt(Function::Alt5).set_gpio_pd(0);

    self.registers.AUX_MU_CNTL_REG.write(0); // Disable auto flow control and disable receiver and transmitter (for now)
    self.registers.AUX_MU_IER_REG.write(1); // Enable receive interrupts and disable transmit interrupts
    self.registers.AUX_MU_LCR_REG.write(3); // Enable 8 bit mode
    self.registers.AUX_MU_MCR_REG.write(0); // Set RTS line to be always high
    self.registers.AUX_MU_BAUD_REG.write(270); // Set baud rate to 115200

    self.registers.AUX_MU_CNTL_REG.write(3); // Finally, enable transmitter and receiver
}
```

### 读

```rust
pub fn has_byte(&self) -> bool {
    self.registers.AUX_MU_LSR_REG.read() & (LsrStatus::DataReady as u8) != 0
}

pub fn read_byte(&self) -> u8 {
    while !self.has_byte() {}
    self.registers.AUX_MU_IO_REG.read()
}
```

### 写

```rust
pub fn write_byte(&mut self, byte: u8) {
    while self.registers.AUX_MU_LSR_REG.read() & (LsrStatus::TxAvailable as u8) == 0 {}
    self.registers.AUX_MU_IO_REG.write(byte);
}
```

## Mailbox

> 参考：https://github.com/raspberrypi/firmware/wiki/Mailboxes

Mailbox 是树莓派上 ARM CPU 与 VideoCore IV GPU 之间通信的渠道。Mailbox 能够识别一段按特定格式存储的请求指令，包含请求代码、请求长度、请求参数等信息，GPU 会根据请求的指令完成相应的操作，并将结果写在原处。

BCM283x 系列有两个 mailbox，一般 MB0 总是用于 GPU 向 CPU 发送消息 MB1 总是用于 CPU 向 GPU 发送消息，对 CPU 来说即一个只读一个只写。

Mailbox 有若干通道(channels)，不同通道提供不同种类的功能。一般使用 property tags 通道(编号为 8)，即 mailbox property interface。

### 基本读写

> 参考：https://github.com/raspberrypi/firmware/wiki/Accessing-mailboxes

对 mailbox 的基本读写实现在 crate [bcm2837](../../../crate/bcm2837/) 的 [mailbox.rs](../../../crate/bcm2837/src/mailbox.rs) 中。一般一次操作是向 mailbox 写入请求的地址，然后读 mailbox 来轮询等待操作完成。注意读写 mailbox 时只有数据的高 28 位有效，低 4 位被用于存放通道，所以如果写入的是一个地址则该地址必须 16 字节对齐。

读的流程如下：

1. 读状态寄存器 `MAIL0_STA`，直到 `MailboxEmpty` 位没有被设置；
2. 从 `MAIL0_RD` 寄存器读取数据；
3. 如果数据的最低 4 位不与要读的通道匹配，则回到 1；
4. 否则返回数据的高 28 位。

```rust
pub fn read(&self, channel: MailboxChannel) -> u32 {
    loop {
        while self.registers.MAIL0_STA.read() & (MailboxStatus::MailboxEmpty as u32) != 0 {}
        let data = self.registers.MAIL0_RD.read();
        if data & 0xF == channel as u32 {
            return data & !0xF;
        }
    }
}
```

写的流程如下：

1. 读状态寄存器 `MAIL1_STA`，直到 `MailboxFull` 位没有被设置；
3. 将数据(高 28 位)与通道(低 4 位)拼接，写入 `MAIL1_WRT` 寄存器。

```rust
pub fn write(&mut self, channel: MailboxChannel, data: u32) {
    while self.registers.MAIL1_STA.read() & (MailboxStatus::MailboxFull as u32) != 0 {}
    self.registers.MAIL1_WRT.write((data & !0xF) | (channel as u32));
}
```

### Mailbox property interface

> 参考：https://github.com/raspberrypi/firmware/wiki/Mailbox-property-interface

Mailbox property interface 提供了丰富的访问底层硬件的接口，包括电源、温度、DMA、GPU、内存、Framebuffer 等模块。RustOS 中封装了一系列 mailbox property interface 函数，实现在 [kernel/src/arch/aarch64/board/raspi3/mailbox.rs](../../../kernel/src/arch/aarch64/board/raspi3/mailbox.rs) 中。

向 mailbox property interface 发送的请求需要符合一定的格式。在 RustOS 中，对 mailbox property interface 的一个功能调用被称为一个 `PropertyMailboxTag`，格式如下：

```rust
#[repr(C, packed)]
struct PropertyMailboxTag<T: Sized> {
    id: PropertyMailboxTagId,
    buf_size: u32,
    req_resp_size: u32,
    buf: T,
}
```

这里的 `buf` 一般是一个 32 位无符号整数的数组。一个或多个 `PropertyMailboxTag` 可构成一个 `PropertyMailboxRequest`，这是最终需要向 mailbox 发送的请求，格式如下：

```rust
#[repr(C, packed)]
struct PropertyMailboxRequest<T: Sized> {
    buf_size: u32,
    req_resp_code: PropertyMailboxStatus,
    buf: T,
    end_tag: PropertyMailboxTagId,
}
```

这里的 `buf` 可以是多个大小不一的 `PropertyMailboxTag` 构成的数组，不过内存布局必须连续而没有空隙。

为了方便构造这两个结构体，定义了宏 `send_one_tag!()` 与 `send_request!()`：

* `send_request!($tags: ident)`：发送一个或多个 `PropertyMailboxTag`。这会构建一个 16 字节对齐的 `PropertyMailboxRequest` 结构体，将其地址写入 mailbox。等待 GPU 操作完毕后，返回被修改过的 `PropertyMailboxTag` 列表。

* `send_one_tag!($id: expr, [$($arg: expr),*])`：这会根据 `id` 与 32 位无符号整数的数组构造一个 `PropertyMailboxTag` 结构体，然后通过宏 `send_request!()` 发送给 mailbox，返回被修改过的数组。

有了这两个宏，就可以非常方便地实现所需的 mailbox property interface 功能了。例如获取 framebuffer 物理大小：

```rust
pub fn framebuffer_get_physical_size() -> PropertyMailboxResult<(u32, u32)> {
    let ret = send_one_tag!(RPI_FIRMWARE_FRAMEBUFFER_GET_PHYSICAL_WIDTH_HEIGHT, [0, 0])?;
    Ok((ret[0], ret[1]))
}
```

`framebuffer_alloc()` 函数是一次性发送多个大小不一的 `PropertyMailboxTag` 的例子。

需要注意的是，当启用 MMU 与 cache 后，在访问 mailbox 的前后都需要刷新整个 `PropertyMailboxRequest` 结构的数据缓存，因为这里涉及到 GPU 与 CPU 的数据共享，必须时刻保证主存与 cache 中数据的一致性。

## Timer

### System Timer

### Generic Timer

## Framebuffer

## Console
