#!/usr/bin/env bash

set -e

DIR="$(cd "$(dirname "$0")" && pwd)"
IMAGE="$DIR/rootfs"

# Pack it
(cd "$IMAGE" && find . | cpio -oH newc > "$DIR/initramfs-linux.img")