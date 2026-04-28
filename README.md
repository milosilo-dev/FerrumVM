![Header](./images/github-header-banner.png)
# Ferrum VMM

A hobby x86 hypervisor written in Rust, with a custom guest firmware written in C, designed to boot Linux via the Limine bootloader.

---

## Overview

Ferrum VMM is a KVM-based virtual machine monitor built from scratch. The goal is to boot a real Linux kernel by implementing the full stack from the reset vector up — including a custom firmware, virtio MMIO device transport, and eventually a Limine-based boot sequence.

The project is split into two halves:

- **Host (Rust)** — the VMM itself. Manages KVM, memory regions, IO/MMIO device dispatch, IRQ routing, and virtio device implementations.
- **Guest (C/asm)** — bare-metal firmware that runs inside the VM. Handles device negotiation, virtqueue management, long mode transition, and will eventually load and jump to Limine.

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
│  └── MMIODeviceMap                          │
│      └── MMIOTransport (virtio)             │
│          ├── RngVirtio  (0x10001000)        │
│          ├── CounterVirtio (0x10002000)     │
│          └── BlkVirtio  (0x10003000)        │
└──────────────────┬──────────────────────────┘
                   │ KVM
┌──────────────────▼──────────────────────────┐
│                 Guest (C/asm)               │
│                                             │
│  entry.asm  → c_main_32() → long mode       │
│  ├── serial_init() / serial_puts()          │
│  ├── virtio_rng_init() / virtio_rng_read()  │
│  ├── virtio_cnt_init() / virtio_cnt()       │
│  └── virtio_blk_init() / virtio_blk_read()  │
└─────────────────────────────────────────────┘
```

---

## Project Structure

```
.
├── src/
│   ├── main.rs                  # VM setup and run loop
│   ├── vm.rs                    # VirtualMachine, KVM wiring, device dispatch
│   ├── vcpu.rs                  # vCPU creation and register init
│   ├── memory_region.rs         # Guest RAM management
│   ├── machine_config.rs        # MachineConfig, MemoryRegionConfig, Binary
│   ├── irq_handler.rs           # IRQ routing and delivery
│   ├── irq_map.rs               # Default IRQ routing table
│   ├── device_maps/
│   │   ├── io.rs                # IO port device map and dispatch
│   │   └── mmio.rs              # MMIO device map and dispatch
│   └── devices/
│       ├── serial.rs            # 16550 UART emulation
│       ├── timer.rs             # PIT 8253 emulation
│       ├── cmos.rs              # CMOS/RTC emulation
│       └── virtio/
│           ├── virtio.rs        # VirtioDevice trait, VirtioQueue, descriptors
│           └── transports/
│               └── mmio.rs      # Virtio MMIO transport (register map, queue wiring)
│           └── devices/
│               ├── rng.rs       # Entropy device (virtio-rng, device ID 0x4)
│               ├── counter.rs   # Counter device (custom, device ID 0x10)
│               └── blk.rs       # Block device (virtio-blk, device ID 0x2)
│
├── guest/
│   └── firmware/
│       ├── entry.asm            # Entry point, protected mode setup, long mode transition
│       ├── main.c               # c_main_32 — top-level guest logic (32-bit)
│       ├── serial.h             # COM1 serial output (outb/inb, serial_puts)
│       ├── types.h              # Freestanding type definitions (uint8_t etc.)
│       ├── gdt.h                # GDT descriptors and GDTR struct definitions
│       ├── paging.h             # PML4/PDPT/PD identity map setup
│       ├── virtio_mmio.h        # MMIO register offsets, mmio_read/write
│       ├── virtqueue.h          # VirtqDesc, VirtqAvail, VirtqUsed, Virtqueue
│       ├── rng.c                # virtio-rng init and read
│       ├── counter.c            # virtio-counter init and request
│       └── blk.c                # virtio-blk init and read
│
├── build.rs                     # Assembles entry.asm, compiles firmware, links binary
└── linker.ld                    # Guest firmware memory layout (loads at 0x7E00)
```

---

## Guest Memory Layout

| Address | Contents |
|---|---|
| `0x7C00` | Guest stack pointer (grows down) |
| `0x7E00` | Guest firmware entry point (`_start`) |
| `0xFFF0` | Reset vector — 5-byte far jump to `0x7E00` |
| `0x10000` | PML4 page table (identity maps first 1 GiB) |
| `0x10001000–0x10001FFF` | virtio-rng MMIO region |
| `0x10002000–0x10002FFF` | virtio-counter MMIO region |
| `0x10003000–0x10003FFF` | virtio-blk MMIO region |

---

## Virtio Devices

### virtio-rng (`0x10001000`)
Standard entropy source. Guest sends a single write-only descriptor, device fills it with random bytes using `vmm_sys_util::rand`. Used to verify the full virtio stack end-to-end.

### virtio-counter (`0x10002000`)
Custom device for learning. Guest sends a `uint32_t` value, device increments it and writes the result back in place. Demonstrates single-descriptor read/write virtio requests.

### virtio-blk (`0x10003000`)
Standard block device. Guest reads sectors from a disk image. Required for MBR parsing and eventually loading Limine.

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

The `build.rs` script automatically assembles `entry.asm`, compiles the C firmware, links it against `linker.ld`, and strips it to a flat binary before the Rust code runs.

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
- [ ] MBR / GPT partition table parser
- [ ] FAT32 / ext2 filesystem reader
- [ ] ELF loader
- [ ] Jump to Limine bootloader
- [ ] Boot Linux