# RISCV 移植记录

## 开发环境

* [riscv-rust/rust](https://github.com/riscv-rust/rust)：使用[官方发布的二进制版本+源码](https://github.com/riscv-rust/rust/releases/tag/riscv-rust-1.26.0-1-dev)
* [riscv-gnu-toolchain](https://github.com/riscv/riscv-gnu-toolchain)：使用OS2018腾讯云中使用的预编译版本

具体配置过程详见[Dockerfile](../riscv-env/Dockerfile)

## BootLoader

参考[bbl-ucore](https://github.com/ring00/bbl-ucore)及后续的[ucore_os_lab for RISCV32](https://github.com/chyyuu/ucore_os_lab/tree/riscv32-priv-1.10)，使用[bbl](https://github.com/riscv/riscv-pk.git)作为BootLoader。

然而官方版本和bbl-ucore中的fork版本都无法正常编译，使用的是[ucore_os_lab中的修改版本](https://github.com/chyyuu/ucore_os_lab/tree/riscv32-priv-1.10/riscv-pk)。

