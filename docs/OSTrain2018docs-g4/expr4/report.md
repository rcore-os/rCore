# Rust OS 教学lab实验的制作 功能完善文档
## 0 添加注释
我们小组对于kernel以及crate中绝大部分重要函数都进行了注释,表明了参数@param,简要功能@brief以及返回值@retval.

此外对于部分代码中比较难以理解的部分和重点部分我们也进行了行内注释.

## 1 内存管理
### 1.1 修改堆内存分配机制
整合了王润基riscv32下对于堆内存分配方式的修改, 改为了在内核end()后间隔一整个页然后其余位置直到
MEMORY_END作为可分配物理内存.

修复了框架中原本的map_kernel时复制的页表项与setup_page_table时建立的页表项不一致的问题,同时增加了qemu设置的物理内存大小(16M)以及给内核堆设置的内存空间(10M),使得OS可以正确执行forktest等占用内存资源较多的用户程序.

### 1.2 page fault 处理
Rust OS riscv32中之前并未实现page fault的异常处理, 目前已经加入page fault处理,目前的page fault 处理程序能够处理的内容包括:
* 虚地址的合法性判断
* 对未分配物理页帧的虚页分配物理页帧并更新页表项
* 页面换出导致的page fault将换出的页面换入
* copy on write导致的page fault处理(由于目前尚未将copy on write机制接入框架,因此正确性有待验证, 这部分内容留作challenge)

### 1.3 物理页帧延迟分配
实现了用户进程memory area部分(非内核部分)的物理内存延迟分配.实现方法是在map的时候对于上述内存区域仅建立页表项,target部分设置为0. 在page fault的时候对于这些页表项分配物理页帧.在进程结束时,仅需将已经分配了的虚拟页对应的物理页释放.

此外在物理页帧延迟分配的过程中,由于需要针对物理地址是否合法进行检查,因此需要保证内核在处理page fault时能够获取当前页表对应的memory set.实际上目前假设出现page fault一定是在进程管理模块已经完成初始化之后,因此大部分情况下当前memory set的可变应用可以通过全局方法process().get_memory_set_mut()来获得,但是当新建一个新进程或者fork一个进程时,当前进程会临时切换页表来进行数据的拷贝和写入,这时触发的page fault显然无法通过process()来获取当前memory_set的可变引用,因此这里用全局变量MEMORY_SET_RECORD来临时保存这些memory set的裸指针,以便page fault处理时可以获得正确的当前page table对应的memory set的可变引用.

### 1.4 页面置换
框架中原本的页面置换算法框架仅支持单页表, 且并未在RustOS中启用而只是在ucore-memory库中进行了单元测试. 此外之前部分swap in/out的底层支持尚未实现(set_swapped等等).

目前已经:

**完成了swap in/out所需的底层支持**: 由于目前框架中尚未启用copy on write, 因此实际上swap并未考虑到copy on write在page entry上所占用的标志位.不过swap标志位仅在not present时候有意义,因此通过略微修改现有框架是可以实现同时开启copy on write和swap 机制的.

**修改页面置换的框架实现，使之支持多个用户进程**: 这里主要的改动在于SwapManager除了需要记录可换出页的虚地址之外,还要记录虚地址所在的二级页表(InactivePageTable0). 在换入换出操作时可能需要短暂进行活跃页表的切换,以便正确执行页表项的修改和虚页内容的写入/移除.

**在RustOS中启用页面置换**: 仅用户进程地址空间的用户页(MemorySet中所包含虚存页,与允许物理页帧延迟分配的虚存一致)允许被换出.由于实现了物理页帧延迟分配操作,因此目前虚页在page fault处理程序中被映射了对应物理页后会被设为可置换.由于1.3中已经完成了随时获取当前的memory_set,在此处可以通过当前的memory_set直接获取当前的不活跃页表并用于页面置换操作.在进程对应的ContextImpl被释放时会首先将其所拥有的memory set中被换出的物理页换入并设置为不可置换,之后再执行Memory Set的释放(Drop)过程.这里需要注意的一点是由于SwapManager是保存的对应不活跃页表的裸指针,而只有new_user以及fork函数执行结束得到的ContextImpl才是保存在堆中的,因此从MEMORY_SET_RECORD获得memory set并由此得到的不活跃页表是不能被保存在SwapManager中的(函数退出后裸指针将会失效).因此在new_user和和fork执行过程中被分配了物理页的虚页会在函数执行结束前被统一制成swappable.

### 1.5 互斥所的替换
将内存管理部分涉及到需要加锁的部分用到的锁从原来的spin::Mutex改为了RustOS框架中实现的sync::SpinNoIrqLock这是为了避免中断可能带来的问题,如切换临时切换页表时被中断或者进行页面换入换出操作时被中断可能导致程序出错.

### 1.6 TODO
* 页面置换Enhanced Clock算法实现: 主要需要修改和完善以适应新的接口, 计划作为lab中的challenge
* 页面换出到磁盘而非堆内存中: ide无法挂载,目前是换入到堆内存中.
* Copy On Write: 之前框架中有Copy on write的部分实现,并未启用,正确性存疑. 计划作为lab中的challenge
* 获取页表项方法中存在的bug修复: 该bug目前不会影响OS运行, 但是这显然是十分危险的(比如在多核时,或者被中断时). 可能的修复方式是fork一份riscv库并对其进行修改,提供对页表项的操作接口.

## 2 进程管理与同步互斥
@陈秋昊

## 3 文件系统
@朱书聪

## 4 测试结果
### 4.1 用户程序测试结果
@刘辰屹 将最开始的RustOS版本与现在版本的用户程序运行结果进行对比

### 4.2 同步互斥测试结果
@陈秋昊

## 5 致谢
感谢王润基同学对于我们完善功能时无私的帮助和指导.
