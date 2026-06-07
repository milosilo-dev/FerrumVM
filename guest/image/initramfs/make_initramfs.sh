#!/usr/bin/env bash

set -e

DIR="$(cd "$(dirname "$0")" && pwd)"
IMAGE="$DIR/rootfs"

# Pack it (use fakeroot to create device nodes and preserve them in cpio archive)
fakeroot bash -c '
  cd "$0"
  # Create device nodes (fakeroot intercepts mknod so they appear in cpio)
  mkdir -p dev
  mknod dev/console c 5 1
  mknod dev/null c 1 3
  mknod dev/ttyS0 c 4 64
  chmod 600 dev/ttyS0
  find . | cpio -oH newc > "$1/initramfs-linux.img"
' "$IMAGE" "$DIR"