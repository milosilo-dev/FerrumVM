#[derive(Clone)]
pub struct IrqMap {
    gsi: u32,
    irq_pin: u32,
    irq_chip: u32,
}

impl IrqMap {
    pub fn new(gsi: u32, irq_pin: u32, irq_chip: u32) -> Self {
        Self {
            gsi,
            irq_pin,
            irq_chip,
        }
    }

    pub fn read_gsi(&self) -> u32 {
        self.gsi
    }

    pub fn read_irq_pin(&self) -> u32 {
        self.irq_pin
    }

    pub fn read_irq_chip(&self) -> u32 {
        self.irq_chip
    }

    pub fn default_map() -> Vec<Self> {
        let mut map = vec![
            Self::new(0, 0, 0), // PIT timer (PIC)
            Self::new(1, 1, 0), // Keyboard (PIC)
            Self::new(3, 3, 0), // COM2 (PIC)
            Self::new(4, 4, 0), // COM1 (PIC)
            Self::new(5, 5, 0), // Virtio-blk (PIC)
            Self::new(6, 6, 0), // Virtio-net (PIC)
            Self::new(7, 7, 0), // Virtio-fs (PIC)
            Self::new(9, 0, 1), // ACPI SCI (PIC slave)
        ];

        for irq_map in map.clone() {
            map.push(Self::new(irq_map.gsi, irq_map.gsi, 2)); // IOAPIC
        }

        map
    }
}

