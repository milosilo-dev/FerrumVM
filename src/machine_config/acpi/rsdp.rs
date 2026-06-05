use crate::machine_config::binary::Binary;

pub fn build_rsdp(xsdt_addr: u64) -> Binary {
    let mut rsdp = vec![0u8; 36];

    rsdp[0..8].copy_from_slice(b"RSD PTR ");
    rsdp[9..15].copy_from_slice(b"FERRUM");
    rsdp[15] = 0x02;
    rsdp[16..20].copy_from_slice(&0u32.to_le_bytes());
    rsdp[20..24].copy_from_slice(&36u32.to_le_bytes());
    rsdp[24..32].copy_from_slice(&xsdt_addr.to_le_bytes());

    rsdp[8] = checksum(&rsdp[0..20]);
    rsdp[32] = checksum(&rsdp[0..36]);

    Binary::new(rsdp, 0xE0000)
}

fn checksum(data: &[u8]) -> u8 {
    data.iter().fold(0u8, |acc, &b| acc.wrapping_add(b))
}
