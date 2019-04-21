# OpenSBI

These are binary release of OpenSBI on this [commit](https://github.com/riscv/opensbi/tree/194dbbe5a13dff2255411c26d249f3ad4ef42c0b) at 2019.04.15.

- fu540.elf: opensbi-0.3-rv64-bin/platform/sifive/fu540/firmware/fw_jump.elf
- virt_rv32.elf: opensbi-0.3-rv32-bin/platform/qemu/virt/firmware/fw_jump.elf
- virt_rv64.elf: opensbi-0.3-rv64-bin/platform/qemu/virt/firmware/fw_jump.elf

NOTE: The [official v0.3 release](https://github.com/riscv/opensbi/releases/tag/v0.3) has bug on serial interrupt.



For K210: It needs some modification. The binary is from this [commit](https://github.com/rcore-os/opensbi/commit/4400a1a7d40b5399b3e07b4bad9fd303885c8c16).

* k210.elf: build/platform/kendryte/k210/firmware/fw_payload.elf