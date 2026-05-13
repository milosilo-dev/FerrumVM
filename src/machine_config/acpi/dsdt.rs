// Load the compiled DSDT file from ACPI and make modifications if needed

use std::fs;

use crate::machine_config::binary::Binary;

pub fn load_dsdt() -> Binary {
    let dsdt = fs::read("acpi/DSDT.aml").unwrap();

    // Patch the binary here in future

    Binary::new(
        dsdt,
        0xE0800,
    )
}