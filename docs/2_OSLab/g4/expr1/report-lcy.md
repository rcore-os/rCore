# Rust OS 实验一分析报告
## 1 环境配置
依据[RustOS开发文档](https://rucore.gitbook.io/rust-os-docs/kai-fa-huan-jing-pei-zhi)中的说明进行安装。编辑器采用VSCode+Rust插件。
成功重现RustOS的交叉编译和在qemu上的x86-64和riscv32的运行步骤重现。

## 2 注释完善
我负责kernel模块的注释完善工作，主要对arch=riscv32从启动到进入shell涉及的代码注释进行了完善，其余部分注释尚未完成有待后续分析完善。

## 3 分析测试
### 3.0 现有测试分析
现有RustOS shell下已经包含了一部分测试程序，测试结果如下(以下测试结果基于arch=riscv32下)：
| 测试程序 | 是否通过 | 错误原因分析 |
| :------: | :------: | :------: |
| waitkill | 通过 |  |
| sleep | 通过 |  |
| spin | 通过 |  |
| sh | 未通过 | unknown syscall id: 0x66, args: [0, 7000ff97, 1, 7000ff70, 25, 73] <br> 可能是由于尚未实现sh这一系统调用 |
| forktest | 通过 |  |
| faultread | 通过 | [INFO] open: path: "stdin:", flags: 0 <br>[INFO] open: path: "stdout:", flags: 1 <br>[ERROR] Process 2 error <br> 系统似乎成功处理了该异常并正确结束该进程？ |
| forktree | 未通过 | PANIC in src/lang.rs at line 22 <br>out of memory<br>似乎是由于“目前所有没被wait过的进程退出后，内存不会被回收”导致的问题|
| divzero | 通过 | |
| yield | 通过 | |
| faultreadkernel | 通过 | 原因同faultread |
| exit | 通过 | |
| softint | 未实现? | |
| badsegment | 通过 | |
| hello | 通过 | |
| ls | 未实现 | |
| priority | 通过 | |
| badarg | 未通过 | PANIC in /home/lcy1996/.rustup/toolchains/nightly-2018-09-18-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/src/libcore/option.rs at line 345 <br> called \`Option::unwrap()\` on a \`None\` value <br> 内核bug?错误原因需要进一步探索|
| testbss | 未通过 | PANIC in /home/lcy1996/.rustup/toolchains/nightly-2018-09-18-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/src/libcore/option.rs at line 1000<br>failed to allocate frame <br>内核bug?错误原因需要进一步探索|
| pgdir | 未通过 | [ERROR] unknown syscall id: 0x1f, args: [a, ffff6ad9, 2, 0, 15, ffffffff] <br> 阅读syscall.rs推测原因是尚未实现0x1f(PGDIR)这个系统调用 |
| matrix | 通过 | |
| sleepkill | 未通过 | PANIC in /home/lcy1996/Documents/OSTrain/RustOS/crate/process/src/event_hub.rs at line 55 <br>    attempt to add with overflow <br> 推测与forktree出错原因相同|

此外现有框架下已经有sync和thread的test但是riscv32下这些test均无法编译通过，询问王润基后推测原因是目前编译器对RISCV原子指令支持不全。因此尝试从ucore lab中移植一些test过来，便于后续lab的裁剪。


