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
        vec![Self::new(0, 0, 0), Self::new(1, 1, 0)]
    }
}
