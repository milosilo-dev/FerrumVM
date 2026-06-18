#!/usr/bin/env bash
set -euo pipefail

CC="x86_64-w64-mingw32-gcc"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

INPUT="${SCRIPT_DIR}/main.c"
RUNTIME="${SCRIPT_DIR}/runtime.c"
OUTPUT="${SCRIPT_DIR}/BOOTX64.EFI"

echo "[*] Compiling EFI application..."

$CC \
    -ffreestanding \
    -fno-builtin \
    -fshort-wchar \
    -mno-red-zone \
    -fno-stack-protector \
    -fno-stack-check \
    -fno-exceptions \
    -fno-asynchronous-unwind-tables \
    -fno-unwind-tables \
    -nostdlib \
    -Wall \
    -Wextra \
    -shared \
    -Wl,--subsystem,10 \
    -Wl,--entry,efi_main \
    "$INPUT" \
    "$RUNTIME" \
    -o "$OUTPUT"

echo "[+] Built $OUTPUT"