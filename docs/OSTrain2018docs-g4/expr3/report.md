# Rust OS 教学lab实验的制作 基础功能完善
## 1 内存管理
@刘辰屹
## 2 进程管理
等待王润基完善后迁移。
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
