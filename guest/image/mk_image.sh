#!/bin/bash
SCRIPT_PATH="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_PATH" || exit

IMG=disk.img
MNT=/mnt/esp

./efi_test/build.sh
cp $1 BOOTX64.EFI

./initramfs/make_initramfs.sh

# 1. Create empty disk (64MB)
dd if=/dev/zero of=$IMG bs=1M count=128

# 2. Create GPT + partition
parted $IMG --script mklabel gpt
parted $IMG --script mkpart ESP fat32 1MiB 64MiB
parted $IMG --script set 1 esp on
parted $IMG --script mkpart ROOT ext4 64MiB 100%

# 3. Attach loop device
LOOP=$(losetup --find --partscan --show $IMG)
partprobe "$LOOP" || true
udevadm settle || true

# 4. Format partition as FAT32
mkfs.fat -F32 ${LOOP}p1
mkfs.ext4 ${LOOP}p2

# 5. Mount it
mkdir -p $MNT
mount ${LOOP}p1 $MNT

# 6. Create EFI structure
mkdir -p $MNT/EFI/BOOT

# 7. Copy bootloader
cp BOOTX64.EFI $MNT/EFI/BOOT/
cp limine.conf $MNT/EFI/BOOT/
cp vmlinuz-linux $MNT
cp initramfs/initramfs-linux.img $MNT

sync
umount $MNT

mount ${LOOP}p2 $MNT

# Install alpine rootfs
cp -a rootfs/. $MNT

sync
umount $MNT

losetup -d $LOOP

echo "Done: UEFI bootable image created"