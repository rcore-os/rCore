# OpenSBI

These are binary release of OpenSBI on this [commit](https://github.com/riscv/opensbi/tree/194dbbe5a13dff2255411c26d249f3ad4ef42c0b) at 2019.04.15.

- virt_rv32.elf: opensbi-0.3-rv32-bin/platform/qemu/virt/firmware/fw_jump.elf
- virt_rv64.elf: opensbi-0.3-rv64-bin/platform/qemu/virt/firmware/fw_jump.elf

NOTE: The [official v0.3 release](https://github.com/riscv/opensbi/releases/tag/v0.3) has bug on serial interrupt. Also, Rocket-Chip based CPUs (including SiFive Unleashed) seem to have unintended behavior on

For K210 & SiFive Unleashed: It needs some modification. The binary is from this [commit](https://github.com/rcore-os/opensbi/commit/a9638d092756975ceb50073d736a17cef439c7b6).

* k210.elf: build/platform/kendryte/k210/firmware/fw_payload.elf
* fu540.elf: build/platform/sifive/fu540/firmware/fw_jump.elf
