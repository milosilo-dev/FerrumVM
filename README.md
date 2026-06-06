![Header](./images/github-header-banner.png)
# Ferrum VMM

A hobby x86 hypervisor written in Rust, with a custom guest firmware written in C, designed to boot Linux via the Limine bootloader.

---

## Overview

Ferrum VMM is a KVM-based virtual machine monitor built from scratch. It boots a real Linux kernel by implementing the full stack from the reset vector up — including a custom firmware, virtio MMIO device transport, ACPI table injection, and a Limine-based boot sequence.

The project is split into two halves:

- **Host (Rust)** — the VMM itself. Manages KVM, memory regions, IO/MMIO device dispatch, IRQ routing, and virtio device implementations.
- **Guest (C/asm)** — bare-metal firmware that runs inside the VM. Handles device negotiation, virtqueue management, long mode transition, Limine boot, and kernel loading via virtio-blk.

---

## Architecture

```
┌─────────────────────────────────────────────┐
│                  Host (Rust)                │
│                                             │
│  VirtualMachine                             │
│  ├── VCPU (KVM vCPU fd)                     │
│  ├── MemoryRegion (mmap'd guest RAM)        │
│  ├── IODeviceMap                            │
│  │   ├── Serial (0x3F8, 0x2F8)              │
│  │   ├── PIT    (0x40–0x43)                 │
│  │   └── CMOS   (0x70–0x71)                 │
│  ├── MMIODeviceMap                          │
│  │   ├── RngVirtio   (0x20000000)           │
│  │   ├── CounterVirtio (0x20001000)         │
│  │   ├── BlkVirtio   (0x20002000)           │
│  │   └── PCI         (0xE0000000)           │
│  └── Tick thread (async device polling)     │
└──────────────────┬──────────────────────────┘
                   │ KVM
┌──────────────────▼──────────────────────────┐
│              Guest (C/asm)                  │
│                                             │
│  entry.asm → c_main_32() → long mode        │
│  ├── ACPI table parsing                     │
│  ├── virtio MMIO device init                │
│  ├── virtio-blk reads (kernel, initramfs)   │
│  └── Limine → Linux kernel boot             │
└─────────────────────────────────────────────┘
```

---

## Project Structure

```
.
├── src/
│   ├── main.rs                  # VM setup and run loop
│   ├── lib.rs                   # Module re-exports
│   ├── vcpu.rs                  # vCPU creation and register init
│   ├── memory_region.rs         # Guest RAM management
│   ├── vm/
│   │   ├── mod.rs
│   │   ├── builder.rs           # VirtualMachine construction
│   │   ├── vm.rs                # VirtualMachine struct
│   │   ├── run.rs               # KVM exit dispatch loop
│   │   └── tick.rs              # Async device tick thread
│   ├── machine_config/
│   │   ├── mod.rs
│   │   ├── machine_config.rs    # MachineConfig, MemoryRegionConfig
│   │   ├── binary.rs            # Binary blob placement
│   │   ├── mem_map.rs           # E820 memory map injection
│   │   └── acpi/                # ACPI table injection (RSDP, XSDT, FADT, DSDT)
│   ├── irq/
│   │   ├── mod.rs
│   │   ├── handler.rs           # IRQ routing and delivery
│   │   └── map.rs               # Default IRQ routing table
│   ├── device_maps/
│   │   ├── io.rs                # IO port device map and dispatch
│   │   └── mmio.rs              # MMIO device map and dispatch
│   └── devices/
│       ├── mod.rs
│       ├── serial.rs            # 16550 UART emulation
│       ├── timer.rs             # PIT 8253 emulation
│       ├── cmos.rs              # CMOS/RTC emulation
│       ├── pci.rs               # PCI config space (placeholder)
│       └── virtio/
│           ├── mod.rs
│           ├── virtio.rs        # VirtioDevice trait, VirtioQueue, descriptors
│           ├── transports/
│           │   ├── mod.rs
│           │   └── mmio.rs      # Virtio MMIO transport (register map, queue wiring)
│           └── devices/
│               ├── mod.rs
│               ├── rng.rs       # Entropy device (virtio-rng)
│               ├── counter.rs   # Counter device (custom)
│               └── blk.rs       # Block device (virtio-blk)
│
├── guest/
│   └── firmware/
│       ├── main.c               # 32-bit entry firmware (c_main_32)
│       ├── main64.c             # 64-bit firmware (Limine boot, Linux load)
│       ├── entry.asm            # Entry point, protected mode → long mode
│       ├── tss.c / tss.h        # Task state segment
│       ├── mem/                 # Memory map helpers
│       ├── headers/             # Firmware header definitions
│       ├── assembly/            # Assembly helpers
│       ├── virtio/              # Virtio MMIO driver (guest side)
│       ├── efi/                 # EFI-related definitions
│       ├── disk/                # Disk read helpers
│       ├── linkerscript/        # Linker scripts
│       └── build/               # Build artefacts
│
├── build.rs                     # Assembles firmware, links, strips binary
├── linker.ld                    # (legacy) Guest firmware memory layout
├── Cargo.toml
└── README.md
```

---

## Guest Memory Layout

| Address | Contents |
|---|---|
| `0x7C00` | Guest stack pointer (grows down) |
| `0x7E00` | Guest firmware entry point (`_start`) |
| `0xFFF0` | Reset vector — far jump to `0x7E00` |
| `0x100000` | 64-bit firmware (`main64.bin`) |
| `0x20000000–0x20000FFF` | virtio-rng MMIO region |
| `0x20001000–0x20001FFF` | virtio-counter MMIO region |
| `0x20002000–0x20002FFF` | virtio-blk MMIO region |
| `0xE0000000–0xE1000000` | PCI MMIO region |

---

## Virtio Devices

### virtio-rng (`0x20000000`)
Standard entropy source. Guest sends a single write-only descriptor, device fills it with random bytes. Used to verify the full virtio stack end-to-end.

### virtio-counter (`0x20001000`)
Custom device for learning. Guest sends a `uint32_t` value, device increments it and writes the result back in place. Demonstrates single-descriptor read/write virtio requests.

### virtio-blk (`0x20002000`)
Standard block device. Guest reads sectors from a disk image (`guest/image/disk.img`). Used to load the kernel and initramfs via Limine.

---

## Building

### Prerequisites

```bash
# Rust toolchain
curl https://sh.rustup.rs -sSf | sh

# Guest firmware toolchain
sudo apt install gcc-multilib binutils nasm
```

### Build and run

```bash
cargo run
```

The `build.rs` script automatically assembles `entry.asm`, compiles the C firmware, links it, and strips it to a flat binary before the Rust code runs.

---

## Roadmap

- [x] KVM vCPU setup and run loop
- [x] IO device dispatch (serial, PIT, CMOS)
- [x] MMIO device dispatch
- [x] Virtio MMIO transport
- [x] Guest firmware — entry, serial output
- [x] virtio-rng
- [x] virtio-counter (custom learning device)
- [x] virtio-blk
- [x] Long mode (64-bit) transition
- [x] ACPI table injection
- [x] PCI configuration space
- [x] Limine bootloader integration
- [x] Linux kernel boot
- [ ] Robust disk image support
- [ ] Interrupt-driven virtio
- [ ] SMP support
- [ ] Networking (virtio-net)
