How to use u-boot to boot rCore in Raspberry Pi
===============

Tested under QEMU.

Instructions:

1. Build u-boot
   1. Download aarch64 toolchain and u-boot source
   2. `make rpi_3_defconfig ARCH=arm CROSS_COMPILE=aarch64-elf-`
   3. `make all ARCH=arm CROSS_COMILE=aarch64-elf-`
   4. A file named `u-boot.bin` should be generated
2. Use u-boot to run rCore
   1. `make run arch=aarch64 u_boot=/path/to/u-boot.bin`
   2. In u-boot, enter following commands:
      1. `mmc read 0x1000000 0 ${nblocks}`, where ${nblocks} can be probed if you enter a large enought number
      2. `bootelf -p 0x1000000`
   3. rCore should boot now