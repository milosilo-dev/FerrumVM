#[derive(Clone)]
pub enum MemType {
    // Used by UEFI
    Reserved            = 0,
    LoaderCode          = 1,
    LoaderData          = 2,
    BootServicesCode    = 3,
    BootServicesData    = 4,
    RuntimeServicesCode = 5,
    RuntimeServicesData = 6,
    ConventionalMemory  = 7,
    Unusable            = 8,
    ACPIReclaimMemory   = 9,
    ACPIMemoryNVS       = 10,
    MMIO                = 11,
}

pub struct MemMap{
    pub start: u64,
    pub end: u64,
    pub mem_type: u32,
}

impl MemMap {
    pub fn as_bytes(&self) -> Vec<u8> {
        let mut start = self.start.to_le_bytes().to_vec();
        let end = self.end.to_le_bytes().to_vec();
        let mem_type = self.mem_type.to_le_bytes().to_vec();

        start.extend_from_slice(&end);
        start.extend_from_slice(&mem_type);

        start
    } 
}

pub struct MemMapHeader{
    pub mgk_num: u32,
    pub length: u32
}

impl MemMapHeader {
    pub fn as_bytes(&self) -> Vec<u8> {
        let mut mgk_num = self.mgk_num.to_le_bytes().to_vec();
        let length = self.length.to_le_bytes().to_vec();

        mgk_num.extend_from_slice(&length);
        mgk_num
    } 
}