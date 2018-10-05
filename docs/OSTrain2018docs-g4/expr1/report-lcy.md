# Rust OS 实验一分析报告
## 1 环境配置
依据[RustOS开发文档](https://rucore.gitbook.io/rust-os-docs/kai-fa-huan-jing-pei-zhi)中的说明进行安装。编辑器采用VSCode+Rust插件。
成功重现RustOS的交叉编译和在qemu上的x86-64和riscv32的运行步骤重现。

## 2 注释完善
我负责kernel模块的注释完善工作，主要对arch=riscv32从启动到进入shell涉及的代码注释进行了完善，其余部分注释尚未完成有待后续分析完善。

## 3 分析测试
### 3.0 现有测试分析
现有框架下已经有sync和thread的test但是riscv32下这些test均无法编译通过，询问王润基后推测原因是目前编译器对RISCV原子指令支持不全。因此尝试从ucore lab中移植一些test过来，便于后续lab的裁剪。
