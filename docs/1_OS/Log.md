## 日志

这里以天为单位，以周为周期，按时间倒序维护了开发日志。

### 第15周

#### 2018.06.06：FS-Process

- 学习xv6/uCore中文件系统上层接口
- 尝试在Rust中实现RootFS和FileManager

#### 2018.06.04：完成同步互斥

- 完成了信号量、条件变量，哲学家就餐问题
- 简易实现了std::sync::mpsc中的FIFO消息传递通道

#### 2018.06.03：新的物理帧分配器

- 实现了新的物理帧分配器，支持回收。其本质是一个0-2^k的整数分配器，内部用bitset维护，并形成树状级联结构，每16bit对应上层1bit，用x86专有指令bsr（相当于整数log2）实现快速分配。目前使用容量为64Kb的分配器，可维护256MB内存空间，实际占用内存9KB以内。
- 由于Rust语言的限制，该类无法实现const fn构造函数，只能使用lazy_static等tradeoff进行运行时初始化。这时必须开启至少O1优化（使用RVO返回值优化，防止在栈上构造再复制到全局变量），否则会栈溢出。

#### 2018.06.02：重构内存管理模块

- 大幅调整了内存管理模块，对外统一使用MemorySet管理一个线程的内存，简化了使用方式。

#### 2018.06.01：同步互斥进阶

- 完成了OS内用锁实现的哲学家就餐问题，和OS外用条件变量的实现。
- 关于Rust并发模型：Rust的所有权机制使得它可以轻松支持【锁数据】而不是【锁代码】。这使得在实现上述哲学家就餐问题时，用锁非常自然，而用条件变量（管程）则不太直观。

### 第14周

#### 2018.05.31：同步互斥初步

* 将spin::Mutex代码fork了一份到内核中，在其中实现了一个接口框架，可以自由替换底层支持。在此框架下，已经实现了自旋锁(spin_lock)、禁止中断的自旋锁(spin_lock_irqsave)，接下来就可以实现自动调度的锁了。
* 实现了禁止中断自旋锁后，即可打开syscall的中断了。
* 为了方便内核态的线程操作，将原有的process模块又封装了一层thread接口，长得和标准库一模一样（std::thread）。这样具体的同步互斥问题（如哲学家就餐）可以先在外部环境中依赖std库实现并测试，然后方便地移植到OS环境中。
* 在此过程中又学习了关于内核抢占、同步互斥锁等相关资料，感觉又是个大坑。并且发现我实现的RustOS一直是内核可抢占的，主要依赖时钟中断进行线程切换，不知要不要改掉。

### 第13周

本周完成的主要工作： 

- 重新实现中断处理入口 
- 实现内核态switch和调度器 
- 从disk0中读取用户程序 
- 支持运行大部分ucore用户程序

#### 2018.05.24 / 25：最终报告

#### 2018.05.23：实现了调度器，IDE驱动

- 参考uCore，实现了RRScheduler和StrideScheduler，并可以正确执行priority程序。 
- 直接Copy了隔壁15组port的ucore的IDE驱动代码，略作修改后就可以使用了。 
- 为SFS模块增加了一个BlockedDevice接口，为它实现Device接口（即支持以字节为单位读写），方便对接真实设备。

#### 2018.05.22：实现了xv6/ucore中的switch()，调整了切换进程的方式

- 为了实现调度器，我阅读了xv6的文档，结果发现自己之前对它切换进程方式的理解都是错的！[捂脸] （长教训，以后一定硬着头皮读懂已有代码，不能闭门造车233） 
- 之前：每次中断后运行的内核态代码和内核栈是不可切换的，切换进程的方法是在结束中断处理时，指定新的rsp地址，这样就可以从别的中断帧恢复环境。这样带来的限制是像sys_wait这种异步操作实现起来很麻烦。 
- 有了switch()之后，就可以自由地在内核线程之间切换。我今天实现了这个，并用它简化了sys_wait。 
- 引入switch()后，要运行一个新的线程变得更加复杂，初始的内核栈上要有TrapFrame+Context。此外我发现调用switch进入scheduler线程，再switch到其它线程，这一过程好像用中断也可以实现（类似上述第2条描述的方法）。

#### 2018.05.21：可以正常跑uCore大部分的用户程序

- 实现了在用户程序发生异常时kill掉它，又支持了一批uCore的用户程序。有了昨天统一中断处理的基础，这个就很容易了。

- 使用mksfs将xv6的64位用户程序也做成了SFS磁盘，将它和ucore程序的SFS磁盘一起添加到git中。

- 修复了Release模式下SMP启动的Bug，添加了一些volatile。

- 修复了waitpid，可以正常跑exit和sleep了。

  注：此时还不支持内核线程的切换（详见05.22的说明），所以(int*)store要先保存下来，不太自然。

#### 2018.05.20：可以正常跑uCore一半的用户程序

写了一个简易事件处理器，支持定期调度+唤醒线程

- 最初实现的是【推模式】，逻辑比较复杂，用户提供回调函数，当事件发生时自动调用。由于Rust的安全约束非常严格，为了通过编译，代码非常复杂。
- 最后实现的是【拉模式】，逻辑比较简单，用户只提供事件内容，在每个时刻都主动询问当前是否有事件，并做处理。这种方式下事件处理器很轻巧，可以内嵌在使用者中，调用关系简洁。

重写了中断处理入口

- 原来使用Redox的处理方式：
  - 每个中断有不同的入口，保存的中断帧可以有不同形式（性能考虑？）。 
  - 但现在需要每种中断都能够进行进程切换，这就需要一个统一的中断帧和处理过程。
- 现在改用ucore/xv6的处理方式：
  - 建立中断向量表vectors，补齐错误码，跳转到统一的处理函数alltraps。 
  - ucore/xv6中vectors.S是用一个perl脚本生成的，在Rust中改用build.rs生成。 
  - 如此修改后，逻辑更加简洁清晰，而且修复了一个长期阴魂不散的Bug

精简了IDT的代码

- 最初使用x86_64库的IDT结构；之后由于类型不匹配的原因，改用Redox的代码 
- 现在随着我姿势水平的提高，学会了绕过类型约束的unsafe黑魔法，重新用起了x86_64库，砍掉了100行的基础代码 
- 历史总是螺旋式上升的，马克思诚不我欺也

Travis上编译问题

- 目前在macOS和docker上均可通过编译 
- 但在Travis上会出现链接问题，不知如何解决

#### 2018.05.19：引入log模块，支持向终端输出彩色文字

- 可以用五种不同级别输出调试信息：error，warn，info，debug，trace 
- 它们在终端以不同颜色显示，使用了Linux终端控制符

#### 2018.05.18：可以从sfs.img中加载用户程序

- 将sfs.img做成.o硬链接到kernel，配合之前写好的SFS模块读取数据

### 第12周

本周完成的主要工作：

- 可以运行用户程序（xv6的64位程序 + ucore的32位程序）

#### 2018.05.18：可以在CLion中配合gdb调试

- x86_64的QEMU在gdb链接时会报错：Remote 'g' packet reply is too long 
- 其实《Writing an OS in Rust》已经给出了[解决方案](https://os.phil-opp.com/set-up-gdb/)。然而我build gdb时编译出错。。 
- 最后解决方案：（macOS下）使用brew安装 altkatz/gcc_cross_compilers/x64-elf-gcc。这个自带上述bug的补丁，在CLion中配置使用这个gdb即可。

#### 2018.05.17：可以运行ucore的32位用户程序

- 不能直接在Long Mode下跑，因为有些指令是环境相关的，例如push 0x64会使得esp -= 8 
- 因此需要进入Compatibility Mode，这方面资料好像很少。[OSDev的一个贴子](https://forum.osdev.org/viewtopic.php?f=1&t=24594)指出只需载入32位的CS即可，即修改L-Bit，但经测试无效，OS会将其识别为16位CS。之后我把xv6的UCODE和UDATA 32位段描述符直接复制过来，就可以正常跑了。经测试，DS也需要是32位的。 
- 目前已经实现了若干ucore系统调用，可以跑hello程序了

#### 2018.05.14：Shared-memory & Copy-on-write

- 利用Rust的Trait特性，将扩展代码都写在一个文件中：[代码](https://github.com/wangrunji0408/RustOS/blob/dev/src/arch/x86_64/paging/cow.rs)。同时包含文档和测试。

#### 2018.05.13：可以运行用户态程序

- 可直接运行xv6 x86_64中的用户程序（二进制兼容），下一步兼容ucore的32位用户程序 
- 实现了一个最简单的syscall：write，可将字符输出到屏幕上 
- 试图实现fork时遇到bug…

#### 2018.05.12：添加MemorySet / Area结构

- 对应ucore中的mm&vma，用来描述虚拟内存集/段 
- 将【内核页表重映射 remap_the_kernel】过程用此结构重构 
- 为下一步加载用户态程序做准备

### 第11周

本周完成的主要工作： 

- 通过兼容层将Rust SFS对接到ucore_os_lab上

#### 2018.05.07：完成与ucore_os_lab的对接

- [总结报告](https://github.com/wangrunji0408/SimpleFileSystem-Rust/blob/master/docs/rust_port_report.md)（后半部分） 
- [整合后的ucore](https://github.com/wangrunji0408/ucore_os_lab/tree/rust-fs/labcodes_answer/lab8_result)

#### 2018.05.06：把SFS链接到uCore

将Rust lib链接到ucore遇到的问题：

- 真的痛苦，估计得掉层皮 
- 遇到最多的是链接丢失问题。ucore_os_lab的linker script少写了一些section，包括`*.data.*`，`*.got.*`，`*.bss.*`，导致Rust lib链接过去后丢失了一些段，比较坑的是这不会有任何提示。没有成功重定位的地址都是0，运行时直接Page fault。为了找出出错位置，各种gdb，objdump全上了，还得看汇编追踪寄存器，真是大坑。 
- 另一个小问题是Rust lib会引用一些LLVM内置函数（如udivdi3，都是除法运算相关），链接时会报undefined symbol。但实际上并没有代码用到它们。《Writing an OS in Rust》中提到了这个问题，它的解决方案是链接时加--gc-sections选项，将未用到的段删掉，结果我对ucore如此操作之后所有段都没了，都boot不起来。。。最后是在C中强行定义这些符号解决的。

关于ucore VFS兼容层的设计：

- ucore VFS中fs和inode头部是具体FS的struct。Rust VFS使用Rc指针相互引用。为了将它们合并起来，考虑过两种方案：在头部放Rc指针，或放Rust SFS的结构本体。前者相当于将Rust VFS作为ucore SFS，后者则是直接将Rust VFS合并到ucore VFS。
  - 放指针：还需建立Rust INode => ucore INode的反向引用，要么侵入式地增加Rust INode的字段，要么在兼容层搞一个全局Map。这种方式耦合较低，但多一层指针跳转，性能可能略差。
  - 放结构：需要在Rust new出SFS结构时做文章， 委托ucore分配多一点空间并做VFS的初始化，得魔改Rust的全局内存分配器。这种方式耦合较高，需对Rust结构的实际内存布局有深入理解。 
- 根据【把方便留给别人，把困难留给自己】【最大兼容，最小耦合】的原则，我选了放指针的方案，目前还在Debug中`_(:3」∠)_`

### 第10周

本周完成的主要工作： 

- 进程模块：只实现了内核线程切换 
- 文件系统：作为独立模块实现完毕，接下来尝试链接到ucore lab8

#### 2018.05.04 / 05：SFS文件系统

- 基本功能实现完毕，附有单元测试 
- 正在尝试和ucore lab8链接 
- 阶段性[移植报告](https://github.com/wangrunji0408/SimpleFileSystem-Rust/blob/master/docs/rust_port_report.md)

#### 2018.04.29 / 05.01：SFS文件系统

[GitHub仓库](https://github.com/wangrunji0408/SimpleFileSystem-Rust)

基础结构移植完毕，各项功能正在重写，计划导出C接口兼容ucore，预计1k行代码可搞定

#### 2018.04.26 / 27：内核线程切换

- 实现了简单的内核线程切换

  每个线程拥有一个[内核栈]，它是当线程运行中发生中断时内核使用的栈，保存着线程的[上下文]信息（中断帧）。对于内核线程而言，其[运行栈]和[内核栈]是统一的。 

  与ucore不同的是：

  - 每个线程的内核栈上只保存自己的[上下文]，调度器不能修改它的内容。且[上下文]只需保存这一份，调度器也不需要访问它的内容。
  - 恢复[上下文]不使用汇编，而是依靠[中断服务例程]结束时从[内核栈]中pop[中断帧]。

  为了实现这点，[中断服务例程]保存完[中断帧]后，把当前rsp传给调度器，调度器将其保存，并用下一个待执行线程的rsp更新之（在最后一次从此线程切出时保存）。[中断服务例程]结束时，重置rsp，切换到另一个线程的[内核栈]上，恢复[中断帧]。

- 实现了int触发内核态和用户态切换（lab1 challenge）

  进入用户态前，需在TSS中设置返回内核态时的rsp。FIXME：初始化多核后，会导致设置失效。

  确认以下设置，否则会出现GPF：

  - 段描述符DPL=3

  - 页表中设置相应页为[用户可访问]

### 第9周

本周应付期中考试、大作业等事务，进展不大。

#### 2018.04.25：页置换算法

实现“改进的时钟置换算法”

### 第8周

本周完成的主要工作： 

- 将Kernel虚地址移到高区 
- 完成lab1的移植，主要是各种驱动 
- 完成多核的初始化 

接下来可以进行的工作： 

- 完善内存管理：完善allocator，mm&vma结构，页替换算法 
- 其它驱动的移植，例如IDE（Rucore组） 
- 进程管理模块

#### 2018.04.19：内存模块

在外部模块定义了页替换模块接口，实现了FIFO算法，并实现了Mock页表用来单元测试。

#### 2018.04.18：多核启动

完成多核AP的启动和初始化：LocalAPIC，GDT，IDT

#### 2018.04.17：设备中断

完成以下设备初始化和中断：串口，键盘，PIT时钟

#### 2018.04.15： 高地址

完成高地址修改，合并到主分支

- （2天前）页表重映射后崩溃的原因，是新页表中权限设置错误（blog_os根据kernel.elf中各段的属性来设置页的权限），使用objdump查看elf的段信息后确认有问题，根本原因是linker script写的不对：我使用rodata32/bbs32/text32来标记BootLoader中的各段，以把它们和Kernel中的区别开，但这导致ld无法正确设置它们的属性，某些应该可写的段被设置为只读，最终造成PageFault。解决方案是把名称改成rodata.32/bss.32/text.32。
- [参考教程](https://wiki.osdev.org/Higher_Half_x86_Bare_Bones)
- 【新技能】在没有初始化中断时，出现TripleFault的Debug方法：利用qemu！
  - 加入参数-d int，即可显示每次中断时的CPU信息
  - 发生PageFault时，检查RPI（出错位置）和CR2（访存目标）

#### 2018.04.14：设备初始化

- 页表建立临时映射，以满足各设备初始化时访问特殊物理地址的需求。TODO：撤除or直接映射全部设备空间 
- 完成以下初始化：LocalAPIC（链接C），IOAPIC，GDT（导入xv6 x86_64的描述符），PIC（复制Redox）
- PIC和APIC都实现了产生时钟中断
  - [关于实现中断的问题手册](https://wiki.osdev.org/I_Cant_Get_Interrupts_Working)

#### 2018.04.13：高地址

尝试将Kernel虚地址移到高地址区，遇到很多问题

- 需要修改linker将kernel虚地址置为高地址区，但实地址还在低地址区（AT指令）
- 需要修改初始页表，将四级页表项的1st和510th同时映射到低1GB物理空间。由于BootLoader执行时PC使用实地址，因此在进入Rust之后才能把1st页表项撤销掉。
- 页表重映射（remap_the_kernel）修改后还没有调试成功，重置CR3的瞬间会崩掉
- 经过测试，初始化IDT只能在页表重映射后进行（放到前面会直接崩掉），在开启IDT前不好Debug。

### 第7周

#### 2018.04.12：中期汇报文档和PPT

- [中期汇报文档](https://github.com/wangrunji0408/RustOS/blob/dev/docs/MidReport.md)
- [中期汇报PPT](https://github.com/wangrunji0408/RustOS/blob/dev/docs/MidPresentation.pdf)

#### 2018.04.11：学习xv6，C语言绑定

- 阅读xv6代码
- 实现了Rust对C的绑定
  - 直接extern符号链接即可，不需要bindgen
  - bindgen输出的代码中，把std::raw::*替换成原生类型即可
- RISCV Rust Toolchain：找到了一个日本人写的[采坑系列文章](http://msyksphinz.hatenablog.com/entry/2017/11/29/021030)，写于2017.12，成功在HiFive上跑起来了。

#### 2018.04.09：TravisCI，C语言绑定

- 从内部退出QEMU的方法：
  - 运行qemu时加入 -device isa-debug-exit
  - 执行outb(0x501, k)，会退出qemu，错误码为2k+1
- [在Travis中运行qemu的方法](https://github.com/jdub/travis-qemu-example)
- 结合上述方法可以在Travis上做集成测试 
- qemu环境下单测比较困难，rust自带的测试框架也无法使用
- 尝试Rust FFI（C语言绑定）
  - [Rust-Bindgen](https://rust-lang-nursery.github.io/rust-bindgen/)
  - 已经对xv6生成了绑定 
  - 但它依赖std库，不好集成到只依赖core的RustOS中 
  - 另一个移植思路是用C绑定Rust，把原有的模块逐个用Rust重写，对C提供临时接口，保持OS始终完整

#### 2018.04.07：ACPI

初步完成ACPI的移植

之后的底层驱动部分，考虑 找Rust库 > 从RustOS里摘 > 从xv6移植。

### 第6周

#### 2018.04.04：RISCV

找到了RISCV Rust Toolchain的Docker，编译样例项目失败，看上去除了作者本人，还没有别人成功过。

#### 2018.04.02：32位boot

在blog_os x86_64框架下实现x86的boot

#### 2018.04.01：RISCV

构建RISCV32/64 Docker

尝试构建RISCV Rust Toolchain，失败