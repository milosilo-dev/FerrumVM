use crate::machine_config::binary::Binary;

pub fn build_fadt(dsdt_addr: u64) -> Binary {
    let mut fadt = vec![0u8; 244]; // FADT length (ACPI 3.0)

    // Header (36 bytes)
    fadt[0..4].copy_from_slice(b"FACP"); // Signature
    fadt[4..8].copy_from_slice(&(244u32).to_le_bytes()); // Length
    fadt[8] = 2; // Revision (ACPI 3.0)
    fadt[9] = 0; // Checksum (calculated later)
    fadt[10..16].copy_from_slice(b"FERRUM"); // OEM ID
    fadt[16..24].copy_from_slice(b"FVM_FADT"); // OEM Table ID
    fadt[24..28].copy_from_slice(&(1u32).to_le_bytes()); // OEM Revision
    fadt[28..32].copy_from_slice(b"FVM "); // Creator ID
    fadt[32..36].copy_from_slice(&(1u32).to_le_bytes()); // Creator Revision

    // Firmware Control (FACS) - 0, no FACS used
    fadt[36..40].copy_from_slice(&0u32.to_le_bytes());

    // DSDT (32-bit address)
    fadt[40..44].copy_from_slice(&(dsdt_addr as u32).to_le_bytes());

    // SCI_INT
    fadt[46..48].copy_from_slice(&9u16.to_le_bytes());

    // SMI command port / ACPI enable-disable writes (QEMU-style, unused by guest)
    fadt[48..52].copy_from_slice(&0x000000B2u32.to_le_bytes()); // SMI_CMD
    fadt[52] = 0xF1; // ACPI_ENABLE
    fadt[53] = 0xF0; // ACPI_DISABLE

    // Power management block IO ports (QEMU PIIX-style)
    fadt[56..60].copy_from_slice(&0x0600u32.to_le_bytes()); // PM1a_EVT_BLK
    fadt[64..68].copy_from_slice(&0x0604u32.to_le_bytes()); // PM1a_CNT_BLK
    fadt[76..80].copy_from_slice(&0x0608u32.to_le_bytes()); // PM_TMR_BLK
    fadt[80..84].copy_from_slice(&0x0620u32.to_le_bytes()); // GPE0_BLK

    fadt[88] = 4; // PM1_EVT_LEN
    fadt[89] = 2; // PM1_CNT_LEN
    fadt[91] = 4; // PM_TMR_LEN
    fadt[92] = 0x10; // GPE0_BLK_LEN

    // RTC device registers (CMOS)
    fadt[106] = 0x0D; // RTC Day Alarm Index
    fadt[108] = 0x32; // RTC Century Index

    // IAPC Boot Flags = 0 (legacy devices + MSI supported)
    fadt[109..111].copy_from_slice(&0u16.to_le_bytes());

    // Flags: WBINVD (bit 0), WBINVD_FLUSH (bit 1)
    fadt[112..116].copy_from_slice(&0x03u32.to_le_bytes());

    // Reset Register
    fadt[116] = 1;     // System I/O
    fadt[117] = 8;     // Bit width
    fadt[118] = 0;     // Bit offset
    fadt[119] = 1;     // Byte access

    fadt[120..128].copy_from_slice(&0x0CF9u64.to_le_bytes());

    fadt[128] = 0x0A;  // Reset value

    // X_FACS (64-bit) - 0, no FACS used
    fadt[132..140].copy_from_slice(&0u64.to_le_bytes());

    // X_DSDT (64-bit address)
    fadt[140..148].copy_from_slice(&dsdt_addr.to_le_bytes());

    // Calculate Checksum so the whole table sums to 0
    let mut sum: u8 = 0;
    for byte in &fadt {
        sum = sum.wrapping_add(*byte);
    }
    fadt[9] = sum.wrapping_neg();

    Binary::new(fadt, 0xE0400)
}