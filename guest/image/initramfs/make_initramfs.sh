#!/usr/bin/env bash

set -e

DIR="$(cd "$(dirname "$0")" && pwd)"
IMAGE="$DIR/image"

# Create the directory structure
mkdir -p "$IMAGE"/{bin,sbin,etc,proc,sys,dev,tmp,newroot}

cp "$DIR/busybox" "$IMAGE/bin/busybox"

# Create symlinks
ln -s busybox "$IMAGE/bin/sh"
ln -s busybox "$IMAGE/bin/mount"
ln -s busybox "$IMAGE/sbin/switch_root"

# Create init script
cat > "$IMAGE/init" << 'EOF'
#!/bin/sh

mount -t proc none /proc
mount -t sysfs none /sys
mount -t devtmpfs none /dev

mount /dev/sda2 /newroot

exec switch_root /newroot /sbin/init
EOF

chmod +x "$IMAGE/init"

# Pack it
(cd "$IMAGE" && find . | cpio -oH newc | gzip > "$DIR/initramfs-linux.img")