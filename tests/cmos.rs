use ferrumvm::{
    device_maps::io::IODevice,
    devices::cmos::{Cmos, to_bcd},
};

#[test]
fn to_bcd_encodes_decimal() {
    assert_eq!(to_bcd(0), 0x00);
    assert_eq!(to_bcd(9), 0x09);
    assert_eq!(to_bcd(10), 0x10);
    assert_eq!(to_bcd(42), 0x42);
    assert_eq!(to_bcd(59), 0x59);
}

#[test]
fn status_a_encodes_uip_oscillator_and_rate() {
    let mut cmos = Cmos::new();
    cmos.set_uip(true);
    cmos.set_oscillator(2);
    cmos.set_rate(6);
    cmos.output(0, &[0x0A]); // select StatusA
    let a = cmos.input(1, 1)[0];
    // bit7 = 1, bits 6..3 = 2 (0x2), bits 2..0 = 6
    assert_eq!(a, (1 << 7) | (2 << 4) | 6);
}

#[test]
fn status_b_defaults_are_24hr_binary_off() {
    let mut cmos = Cmos::new();
    cmos.output(0, &[0x0B]); // select StatusB
    // default: 24-hour (bit1 = 1), binary=false (bit2=0)
    let b = cmos.input(1, 1)[0];
    assert_eq!(b & (1 << 1), 1 << 1); // 24hr
    assert_eq!(b & (1 << 2), 0); // binary disabled
    // all other flags clear
    assert_eq!(b & 0xFD, 0);
}

#[test]
fn selecting_status_b_and_writing_sets_flags() {
    let mut cmos = Cmos::new();
    cmos.output(0, &[0x0B]); // select StatusB
    cmos.output(1, &[0x84]); // halt + binary
    // reg is still StatusB: read it back directly
    let b = cmos.input(1, 1)[0];
    assert_eq!(b & 0x84, 0x84); // halt (0x80) + binary (0x04)
    assert!(cmos.is_halt());
    assert!(cmos.is_binary());
}

#[test]
fn setting_binary_mode_changes_second_encoding() {
    let mut cmos = Cmos::new();
    // BCD mode: seconds should be BCD-encoded
    let bcd_val = cmos.input(1, 1)[0];
    assert!(bcd_val <= 0x59, "BCD second out of range: {:#x}", bcd_val);

    cmos.output(0, &[0x0B]); // StatusB
    cmos.output(1, &[0x04]); // enable binary
    cmos.output(0, &[0x00]); // select Seconds
    let bin_val = cmos.input(1, 1)[0];
    assert!(bin_val <= 59, "binary second out of range: {}", bin_val);
}

#[test]
fn status_c_is_zero_status_d_battery_good() {
    let mut cmos = Cmos::new();
    cmos.output(0, &[0x0C]);
    assert_eq!(cmos.input(1, 1)[0], 0x00);
    cmos.output(0, &[0x0D]);
    assert_eq!(cmos.input(1, 1)[0], 0x80);
}

#[test]
fn invalid_register_index_keeps_previous_selection() {
    let mut cmos = Cmos::new();
    cmos.output(0, &[0x0B]); // select StatusB
    cmos.output(0, &[0x7F]); // invalid index, & 0x7F = 0x7F, not matched
    assert_eq!(cmos.input(1, 1)[0] & 0x02, 0x02); // still StatusB (24hr bit)
}
