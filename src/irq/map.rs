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
        vec![
            Self::new(0, 0, 0), // PIT timer
            Self::new(1, 1, 0), // Keyboard
            Self::new(3, 3, 0), // COM2 (serial)
            Self::new(4, 4, 0), // COM1 (serial)
            Self::new(5, 5, 0), // Virtio-blk (MMIO)
            Self::new(6, 6, 0), // Virtio-net (MMIO)
        ]
    }
}
