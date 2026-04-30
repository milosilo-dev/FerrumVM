pub enum MemMapType{
    Undefined,
    Usable,
    Reserved,
}

pub struct MemMap{
    pub base: u64,
    pub length: u64,
    pub mem_type: u32,
}

impl MemMap {
    pub fn as_bytes(&self) -> Vec<u8> {
        let mut base = self.base.to_le_bytes().to_vec();
        let length = self.length.to_le_bytes().to_vec();
        let mem_type = self.mem_type.to_le_bytes().to_vec();

        base.extend_from_slice(&length);
        base.extend_from_slice(&mem_type);

        base
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