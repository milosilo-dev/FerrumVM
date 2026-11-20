struct ConfigRegion {
    pub data: Vec<u8>,
    pub base: u16,
    pub writable: bool,
}

pub struct ConfigInstance {
    config_regions: Vec<ConfigRegion>,
}

impl ConfigInstance {
    pub fn new() -> Self {
        Self {
            config_regions: vec![],
        }
    }

    pub fn add_config_region(&mut self, data: Vec<u8>, base: u16, writable: bool) {
        self.config_regions.push(ConfigRegion {
            data,
            base,
            writable,
        });
    }

    pub fn read(&self, addr: u16) -> Option<u8> {
        for config_region in &self.config_regions {
            if config_region.base <= addr
                && config_region.base + config_region.data.len() as u16 > addr
            {
                if let Some(ret) = config_region.data.get((addr - config_region.base) as usize) {
                    return Some(*ret);
                }
            }
        }
        None
    }

    pub fn write(&mut self, addr: u16, data: u8) {
        for config_region in &mut self.config_regions {
            if config_region.base <= addr
                && config_region.base + config_region.data.len() as u16 > addr
                && config_region.writable
            {
                config_region.data[(addr - config_region.base) as usize] = data;
            }
        }
    }
}
