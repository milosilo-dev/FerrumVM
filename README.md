![FerrumVM](./images/github-header-banner.png)

# FerrumVM

[![License: Apache 2.0](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](LICENSE)
![Rust](https://img.shields.io/badge/Rust-1.85+-orange)
![C](https://img.shields.io/badge/C-99-f34b7d)

**FerrumVM** is a hobby x86 KVM-based virtual machine monitor written in Rust, with a custom bare-metal guest firmware in C and assembly. It boots a real Linux kernel by implementing the full boot stack from the x86 reset vector through protected mode, long mode, ACPI table injection, virtio MMIO device transport, and the Limine boot protocol through uefi.

---

## Features

- **Full x86 boot path** — reset vector (0xFFF0) → real mode → protected mode → long mode (64-bit)
- **KVM-based virtualization** — vCPU creation, memory registration, IRQ routing
- **Custom guest firmware** — serial output, paging (PAE, 2MiB huge pages), GDT/IDT/TSS, heap allocator, virtio drivers
- **Virtio MMIO transport** — legacy + modern mode, queue negotiation, IRQ delivery
- **Virtio devices:**
  - `virtio-blk` — block device reading kernel and initramfs from a disk image
  - `virtio-net` — network device over a TAP interface
  - `virtio-rng` — entropy source
  - `virtio-counter` — custom single-descriptor read/write learning device
- **IO device emulation** — 16550 UART (dual COM), PIT 8253, CMOS/RTC
- **ACPI table injection** — RSDP, XSDT, FADT, DSDT with virtio device definitions
- **E820 memory map** — injected form host for guest firmware memory management
- **Limine bootloader integration** — loads Linux kernel and initramfs via EFI boot protocol
- **Async device tick thread** — periodic device polling and IRQ delivery

---

## How It Works

```
┌──────────────────────────────────────────────┐
│                Host (Rust)                    │
│                                               │
│  VirtualMachine                               │
│  ├── VCPU (KVM vCPU fd)                       │
│  ├── MemoryRegion (mmap'd guest RAM)          │
│  ├── IODeviceMap                              │
│  │   ├── Serial (0x3F8, 0x2F8)                │
│  │   ├── PIT    (0x40–0x43)                   │
│  │   └── CMOS   (0x70–0x71)                   │
│  ├── MMIODeviceMap                            │
│  │   ├── RngVirtio   (0x20000000)             │
│  │   ├── CounterVirtio (0x20001000)           │
│  │   ├── BlkVirtio   (0x20002000)             │
│  │   ├── NetVirtio   (0x20003000)             │
│  │   └── PCI         (0xE0000000)             │
│  └── Tick thread (async device polling)       │
└────────────────────┬─────────────────────────┘
                     │ KVM
┌────────────────────▼─────────────────────────┐
│              Guest (C/asm)                     │
│                                                │
│  entry.asm → c_main_32() → long mode           │
│  ├── ACPI table parsing                        │
│  ├── virtio MMIO device init                   │
│  ├── virtio-blk reads (kernel, initramfs)      │
│  └── Limine → Linux kernel boot                │
└────────────────────────────────────────────────┘
```

### Boot Flow

1. x86 CPU starts at the **reset vector** (0xFFF0), which far-jumps to the firmware entry at 0x7E00
2. `entry.asm` transitions from real mode → protected mode → long mode
3. `c_main_32()` sets up paging, GDT, and enters 64-bit mode
4. `c_main_64()` parses ACPI tables, initializes virtio devices, reads the kernel and initramfs from disk via virtio-blk
5. **Limine boot protocol** transfers control to the Linux kernel
6. Linux boots with serial console output via the emulated 16550 UART

---

## Quick Start

### Prerequisites

```bash
# Rust toolchain
curl https://sh.rustup.rs -sSf | sh

# Guest firmware build tools
sudo apt install gcc-multilib binutils nasm iasl

# 64-bit cross-compiler for firmware
sudo apt install gcc-x86-64-linux-gnu
```

> **Note:** The build script also expects `i686-elf-gcc`. Install a cross-compiler for the 32-bit firmware target if needed, or adjust `build.rs` to use a different toolchain.

### Build and Run

```bash
cargo run
```

The `build.rs` script automatically assembles the firmware entry, compiles the C firmware, links and strips it to a flat binary, compiles the ACPI DSDT, and then the Rust VMM code compiles and runs.

### Network (Optional)

For virtio-net support, the host needs TAP device access:

```bash
sudo ip tuntap add dev tap0 mode tap user $(whoami)
sudo ip link set tap0 up
```

---

## Guest Memory Layout

| Address | Contents |
|---------|----------|
| `0x7C00` | Guest stack pointer (grows down) |
| `0x7E00` | Firmware entry point (`_start`) |
| `0xFFF0` | Reset vector — far jump to `0x7E00` |
| `0x100000` | 64-bit firmware (`main64.bin`) |
| `0x20000000` | virtio-rng MMIO region |
| `0x20001000` | virtio-counter MMIO region |
| `0x20002000` | virtio-blk MMIO region |
| `0x20003000` | virtio-net MMIO region |
| `0xE0000000` | PCI MMIO region |

---

## Project Structure

```
src/                          # Host VMM (Rust)
├── main.rs                   # Entry point and run loop
├── lib.rs                    # Module re-exports
├── vcpu.rs                   # vCPU creation and register init
├── memory_region.rs          # Guest RAM (mmap)
├── vm/                       # VirtualMachine core
│   ├── builder.rs            # Construction and KVM setup
│   ├── vm.rs                 # Struct definition
│   ├── run.rs                # KVM exit dispatch
│   └── tick.rs               # Async device polling
├── machine_config/           # Machine configuration
│   ├── machine_config.rs     # Config types
│   ├── binary.rs             # Binary blob placement
│   ├── mem_map.rs            # E820 memory map
│   └── acpi/                 # ACPI table builders
├── irq/                      # IRQ routing
├── device_maps/              # IO/MMIO dispatch
└── devices/
    ├── serial.rs             # 16550 UART
    ├── timer.rs              # PIT 8253
    ├── cmos.rs               # CMOS/RTC
    ├── pci.rs                # PCI config space
    └── virtio/
        ├── virtio.rs         # Core types and traits
        ├── transports/mmio.rs # MMIO transport layer
        └── devices/
            ├── rng.rs        # virtio-rng
            ├── counter.rs    # virtio-counter (custom)
            ├── blk.rs        # virtio-blk
            └── net.rs        # virtio-net

guest/                        # Guest firmware (C/asm)
├── firmware/
│   ├── main.c                # 32-bit entry firmware
│   ├── main64.c              # 64-bit firmware (Limine boot)
│   ├── entry.asm             # Reset vector → protected mode → long mode
│   ├── assembly/             # Assembly stubs and IDT handlers
│   ├── headers/              # Firmware headers
│   ├── mem/                  # Heap and memory map
│   ├── virtio/               # Guest virtio drivers
│   ├── efi/                  # EFI protocol definitions
│   ├── disk/                 # Disk access helpers
│   └── linkerscript/         # Linker scripts
├── image/                    # Disk image and boot config
└── tests/                    # Assembly test programs

build.rs                      # Firmware build script
acpi/                          # ACPI DSDT source
```

---

## Roadmap

### Done

- [x] KVM vCPU setup and run loop
- [x] IO device dispatch (serial, PIT, CMOS)
- [x] MMIO device dispatch
- [x] Virtio MMIO transport
- [x] Guest firmware — entry, serial output, paging, long mode
- [x] virtio-rng
- [x] virtio-counter (custom learning device)
- [x] virtio-blk (kernel and initramfs loading)
- [x] virtio-net (TAP interface, RX/TX queues)
- [x] ACPI table injection (RSDP, XSDT, FADT, DSDT)
- [x] PCI configuration space
- [x] Limine bootloader integration
- [x] Linux kernel boot (Alpine Linux)

### In Progress / Planned

- [ ] Interrupt-driven virtio (vs. polling)
- [ ] Robust disk image support
- [ ] SMP support
- [ ] MSI-X interrupt delivery
- [ ] UEFI firmware support
- [ ] Rust guest firmware (experimental)

---

## License

Licensed under the Apache License, Version 2.0. See [LICENSE](LICENSE).

