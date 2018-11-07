# Rust OS 教学lab实验的制作 基础功能完善
## 1 内存管理
### 1.0 尚未实现
* 页面置换Enhanced Clock算法实现: 主要需要修改和完善以适应新的接口, 可以考虑作为lab中的challenge
* 页面换出到磁盘而非堆内存中: ide无法挂载,目前是换入到堆内存中.
* Copy On Write: 之前框架中有Copy on write的部分实现,并未启用,正确性存疑. 可以考虑作为lab中的challenge
* 获取页表项方法中存在的bug修复: 该bug目前不会影响OS运行, 但是这显然是十分危险的(比如在多核时,或者被中断时). 可能的修复方式是fork一份riscv库并对其进行修改,提供对页表项的操作接口.

### 1.1 page fault 处理
Rust OS riscv32中之前并未实现page fault的异常处理, 目前已经加入page fault处理,目前的page fault能够处理的错误包括:
* 页面换出导致的page fault
* copy on write导致的page fault(由于目前尚未将copy on write机制接入框架,因此正确性有待验证)

### 1.2 页面置换
框架中原本的页面置换算法框架仅支持单页表, 且并未在RustOS中启用而只是在ucore-memory库中进行了测试. 此外之前部分swap in/out的底层支持尚未实现(set_swapped等等).

目前已经:

**完成了swap in/out所需的底层支持**: 由于目前框架中尚未启用copy on write, 因此实际上swap并未考虑到copy on write在page entry上所占用的标志位.不过swap标志位仅在not present时候有意义,因此通过略微修改现有框架是可以实现同时开启copy on write和swap 机制的.

**修改页面置换的框架实现，使之支持多个用户进程**: 这里主要的改动在于SwapManager除了需要记录可换出页的虚地址以为,还要记录虚地址所在的二级页表(InactivePageTable0). 在换入换出操作时可能需要短暂进行活跃页表的切换.

**在RustOS中启用页面置换**: 仅用户进程地址空间的用户页(MemorySet中所包含的地址)允许被换出.目前实现是在用户进程创建时将这些页设为swappable,在进程结束Context资源释放前先将这些页全部换入内存,然后进行unmap操作释放物理内存.

### 1.3 物理页帧延迟分配
实现了用户线程memory area部分(非内核部分)的物理内存延迟分配.实现方法是在map的时候对于上述内存区域仅建立页表项,target部分设置为0. 在page fault的时候对于这些页表项分配物理页帧.目前来说没有处理不合法虚地址的问题,但是目前memory set record记录了new user和fork的时候的临时memory的记录,因此*理论上用裸指针是比较好实现虚地址合法性的判断*(process.get_memory_set_mut()可以获取一般的memory set的可变引用).


## 2 进程管理
已经完成从王润基处的移植,目前的问题是依然会有out of memory.
## 3 同步互斥
修正了原本底层原子函数的bug并补全了底层原子函数的实现，但是目前的底层使用关中断实现，不支持多核，不过不影响lab实验的制作。
## 4 文件系统

由于文件系统的主体不在主仓库而是在 [wangrunji0408/SimpleFileSystem-Rust](https://github.com/wangrunji0408/SimpleFileSystem-Rust) 中，对文件系统的修改在 [benpigchu/SimpleFileSystem-Rust](https://github.com/benpigchu/SimpleFileSystem-Rust) 进行（`ucore-fs-enhance` 分支）。要在主仓库中预览目前进行的修改，可以在 `kernel/Cargo.toml` 中加入以下内容：
```toml
[patch."https://github.com/wangrunji0408/SimpleFileSystem-Rust"]
simple-filesystem = { git = "https://github.com/benpigchu/SimpleFileSystem-Rust", branch="ucore-fs-enhance" }
```

以下是目前的进度与在真正进入教学 Lab 划分与制作前的计划

- [x] 修复原有实现的错误
	- [x] 正确维护和解释磁盘上的 `inode` 结构的 `size` 项
	- [x] 正确维护磁盘上的 `inode` 结构的 `nlinks` 项
- [-] 补充实现之前未实现的功能
	- [x] `unlink`
	- [x] `link`
	- [ ] `mount`
	- [ ] 符号链接相关内容
- [-] 调整接口的定义
	- [x] 用只获得一个目录项的 `get_entry` 取代获得所有目录项的 `list`
	- [x] 用单层查找的 `find` 取代多层查找的 `lookup`
	- [ ] 返回错误类型而非直接 `panic!`
	- [ ] 将 `vfs` `device` 等与具体文件系统无关的内容从 `SimpleFileSystem-Rust` 仓库移动到主仓库的新包中（`crate/vfs`）
- [ ] 让用户程序能够操作文件系统
	- [ ] 使文件系统线程安全
	- [ ] 实现文件描述符
	- [ ] 实现相关系统调用
