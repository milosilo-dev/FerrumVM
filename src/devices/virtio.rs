use crate::device_maps::mmio::MMIODevice;

pub struct VirtioBlock {
    queue_ready: u8,
    interrupt_status: u64,
}

impl MMIODevice for VirtioBlock {
    fn read(&mut self, addr: u64, length: usize) -> Vec<u8> {
        let value = (match addr {
            0x000 => 0x74726976,                // Magic number
            0x004 => 0x2,                       // Version
            0x008 => 0x2,                       // Device block
            0x00C => 0x56484B53,                // Vendor ID
            0x010 => 0x0,                       // Device Fetures
            0x034 => 64,                        // Queue size
            0x044 => self.queue_ready as u64,   // Queue ready
            0x060 => self.interrupt_status,     // IRQ Status
            0x070 => 0,                         // Status
            _ => 0,
        } as u64).to_le_bytes();
        value[..length].to_vec()
    }

    fn write(&mut self, addr: u64, data: &[u8]) {
        todo!()
    }

    fn irq_handler(&mut self, irq_handler: std::sync::Arc<std::sync::Mutex<crate::irq_handler::IRQHandler>>) {
        todo!()
    }

    fn tick(&mut self) {
        todo!()
    }
}