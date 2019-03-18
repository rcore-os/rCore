#!/bin/bash
# tested on Ubuntu 18.04 & macOS 10.14

# To run on macOS, first:
#   brew install gptfdisk

set -e

if [ "x$2" == "x" ]; then
  echo Usage: $0 in out
  exit 1
fi

# Parameters
RAW=$1
IMG=$2
BS=512
RAW_START=2048

# Relevant partition type codes
BBL=2E54B353-1271-4842-806F-E436D6AF6985
FSBL=5B193300-FC78-40CD-8002-E86C45580B47

if [[ `uname` == "Darwin" ]]; then
  M=m
  RAW_SIZE=`stat -f%z ${RAW}`
  type sgdisk || { echo "Try: brew install gptfdisk"; exit 1; }
else
  M=M
  RAW_SIZE=`du -b ${RAW} | cut -d "	" -f1`
fi

echo Input file is ${RAW_SIZE} bytes.

RAW_BLOCKS=$(((${RAW_SIZE} + ${BS} - 1) / ${BS}))
RAW_END=$((${RAW_START} + ${RAW_BLOCKS} - 1))

echo Start=${RAW_START}
echo Blocks=${RAW_BLOCKS}
echo End=${RAW_END}

echo Creating an image file...
dd if=/dev/zero of=${IMG} bs=${BS} count=$((${RAW_END} + 128))

echo Partitioning the image...
sgdisk --clear \
  --new=1:${RAW_START}:${RAW_END} \
  --change-name=1:bootloader \
  --typecode=1:${BBL} \
  -p ${IMG}

echo Writing bootloader into the image...
dd if=${RAW} of=${IMG} conv=notrunc bs=${BS} seek=${RAW_START} count=${RAW_BLOCKS}

echo Done.
echo Use \"dd if=${IMG} of=/dev/XXX\" to write the image to a real SD card.
