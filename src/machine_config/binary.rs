pub struct Binary {
    pub data: Vec<u8>,
    pub offset: u64,
}

impl Binary {
    pub fn new(data: Vec<u8>, offset: u64) -> Self {
        Self { data, offset}
    }
    
    pub fn reset_vector() -> Self {
        Self { data: vec![0xEA, 0x00, 0x7E, 0x00, 0x00], offset: 0xFFF0}
    }
}