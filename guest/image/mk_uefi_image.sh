#!/bin/bash

IMG=disk.img
MNT=/mnt/esp

# 1. Create empty disk (64MB)
dd if=/dev/zero of=$IMG bs=1M count=64

# 2. Create GPT + partition
parted $IMG --script mklabel gpt
parted $IMG --script mkpart ESP fat32 1MiB 100%
parted $IMG --script set 1 esp on

# 3. Attach loop device
LOOP=$(losetup --find --partscan --show $IMG)

# 4. Format partition as FAT32
mkfs.fat -F32 ${LOOP}p1

# 5. Mount it
mkdir -p $MNT
mount ${LOOP}p1 $MNT

# 6. Create EFI structure
mkdir -p $MNT/EFI/BOOT

# 7. Copy bootloader
cp BOOTX64.EFI $MNT/EFI/BOOT/

sync
umount $MNT
losetup -d $LOOP

echo "Done: UEFI bootable image created"