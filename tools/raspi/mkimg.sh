#!/bin/bash

BASE_DIR=$(dirname "$0")
BOOT_BLOCKS=524288
SFS_INDEX=2

usage() {
    echo "Usage: mkimage.sh [-p] [-b boot] [-s sfsimg] [-k kernel] [disk_id | mount_point]
        -p      Partition the disk with MBR partition table and two partitions
                BOOT and SFSIMG.
        -b      Write the raspberry pi boot firmwares in the specified directory
                \`boot\` to the disk.
        -s      Write the SFS image \`sfsimg\` to the disk.
        -k      Write the kernel image \`kernel\` to the BOOT partition of the
                disk.
\`disk_id\` is of the form /dev/rdisk2.
\`mount_point\` is of the form /Volumes/BOOT."
}

query() {
    echo $1
    while true; do
        read -p "Continue? [Y/n]" yn
        case $yn in
            [Yy]* ) return 1 ;;
            [Nn]* ) return 0 ;;
            * ) ;;
        esac
    done
}

find_disk_identifier() {
    disk=$(mount | grep -i "$VOLUME" | awk '{print $1}' | sed "s/s[0-9]*$//" | head -n 1)
    DISK=$(echo "$disk" | sed "s/disk/rdisk/")
}

find_boot_volume() {
    # mount disk first
    diskutil mountDisk $DISK
    # replace '/dev/rdisk' to '/dev/disk'
    disk=$(echo "$DISK" | sed "s/rdisk/disk/")
    # mount point of first partition
    VOLUME=$(mount | grep $disk | awk '{print $3}' | head -n 1)
}

check_partition() {
    output=$(diskutil list $DISK)
    lines=$(echo "$output" | wc -l)
    disk_type=$(echo "$output" | sed -n '3p' | awk '{print $2}')
    part_type=$(echo "$output" | sed -n '4p' | awk '{print $2}')
    if [[ $lines -ne 5 ]] || [[ "$disk_type" != "FDisk_partition_scheme" ]] || [[ "$part_type" != "DOS_FAT_32" ]]
    then
        NEED_PART=1
    fi
}

partition() {
    query "Will rebuild the partition table on disk $DISK."
    if [[ $? -eq 1 ]]
    then
        echo "Partitioning disk..."
        diskutil partitionDisk $DISK MBR FAT32 BOOT ${BOOT_BLOCKS}s FAT32 SFSIMG 0
    fi
}

write_boot() {
    echo "Writing boot firmwares..."
    firmwares="$FIRM_DIR/*"
    boot_dir="$VOLUME"
    cp $firmwares $boot_dir
}

write_sfsimg() {
    volume=${DISK}s${SFS_INDEX}
    query "Will write SFS image to the volume $volume."
    if [[ $? -eq 1 ]]
    then
        echo "Writing SFS image..."
        diskutil unmountDisk $DISK && sudo dd if=$SFSIMG of=$volume conv=sync bs=0x100000
    fi
}

install_kernel() {
    echo "Installing kernel image..."
    boot_dir="$VOLUME"
    cp $KERNEL $boot_dir/kernel8.img && diskutil unmount $boot_dir
}

while getopts "h?pb:s:k:" opt; do
    case "$opt" in
    h|\?)
        usage
        exit 1
        ;;
    p)
        NEED_PART=1
        ;;
    b)
        FIRM_DIR=$OPTARG
        ;;
    s)
        SFSIMG=$OPTARG
        ;;
    k)
        KERNEL=$OPTARG
        ;;
    esac
done
shift "$(($OPTIND-1))"

VOLUME="/Volumes/BOOT"

if [[ ! -z $1 ]]
then
    if [[ $1 =~ "/dev" ]]
    then
        DISK=$1
    else
        VOLUME=$1
    fi
fi

if [[ -z "$DISK" ]] && [[ ! -z "$VOLUME" ]]
then
    find_disk_identifier
elif [[ -z "$VOLUME" ]] && [[ ! -z "$DISK" ]]
then
    find_boot_volume
fi

if [[ -z "$VOLUME" ]] || [[ -z "$DISK" ]]
then
    echo "$0: requires disk_id or mount_point"
    usage
    exit 1
fi

echo "Disk identifier: $DISK"
echo "Mount point: $VOLUME"

check_partition

if [[ $? -eq 0 ]] && [[ ! -z $NEED_PART ]]
then
    partition
fi

if [[ $? -eq 0 ]] && [[ ! -z "$FIRM_DIR" ]]
then
    write_boot
fi

if [[ $? -eq 0 ]] && [[ ! -z "$SFSIMG" ]]
then
    write_sfsimg
fi

if [[ $? -eq 0 ]] && [[ ! -z "$KERNEL" ]]
then
    install_kernel
fi
