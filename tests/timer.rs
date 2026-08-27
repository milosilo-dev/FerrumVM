use ferrumvm::{device_maps::io::IODevice, devices::timer::Pit};

#[test]
fn period_ns_default_divisor_is_65536() {
    // 65536 / 1193182 * 1e9 ns
    let expected = (65536.0 / 1_193_182.0) * 1_000_000_000.0;
    assert!((Pit::period_ns(0) - expected).abs() < 1.0);
    assert!((Pit::period_ns(65536) - expected).abs() < 1.0);
}

#[test]
fn period_ns_scales_with_divisor() {
    let p1 = Pit::period_ns(1000);
    let p2 = Pit::period_ns(2000);
    assert!(p2 > p1);
    assert!(p1 > 0.0);
    // period is linear in divisor: doubling the divisor doubles the period
    assert!((p2 - (2.0 * p1)).abs() < 1.0);
}

#[test]
fn lobyte_only_programs_low_byte() {
    let mut pit = Pit::new();
    // mode/command: channel 0, access=1 (lobyte)
    pit.output(0x3, &[0x10]);
    pit.output(0x0, &[0x50]);
    assert_eq!(pit.divisor(), 0x50);
    assert!(pit.programmed());
}

#[test]
fn hibyte_only_programs_high_byte() {
    let mut pit = Pit::new();
    // channel 0, access=2 (hibyte)
    pit.output(0x3, &[0x20]);
    pit.output(0x0, &[0xA0]);
    assert_eq!(pit.divisor(), 0xA000);
    assert!(pit.programmed());
}

#[test]
fn lobyte_hibyte_programming_accumulates() {
    let mut pit = Pit::new();
    // channel 0, access=3 (lobyte/hibyte)
    pit.output(0x3, &[0x30]);
    pit.output(0x0, &[0x34]); // lobyte
    assert_eq!(pit.divisor(), 0x34);
    assert!(!pit.programmed()); // not fully programmed yet
    pit.output(0x0, &[0x12]); // hibyte
    assert_eq!(pit.divisor(), 0x1234);
    assert!(pit.programmed());
}

#[test]
fn reprogramming_resets_divisor_and_latches() {
    let mut pit = Pit::new();
    pit.output(0x3, &[0x30]);
    pit.output(0x0, &[0x34]);
    // mode/command again resets latch_lo, so next byte is lo again
    pit.output(0x3, &[0x30]);
    assert_eq!(pit.divisor(), 0); // reset
    pit.output(0x0, &[0x01]); // should latch LO, not overwrite hi
    assert_eq!(pit.divisor(), 1);
}

#[test]
fn non_channel_zero_commands_are_ignored() {
    let mut pit = Pit::new();
    // channel 1 command is ignored (channel != 0): no mode reset
    pit.output(0x3, &[0x40]);
    // access_mode/op_mode unchanged from defaults (3/3), latch_lo still true
    assert_eq!(pit.access_mode(), 3);
    assert_eq!(pit.op_mode(), 3);
    // port-0 data still programs in current lobyte/hibyte mode
    pit.output(0x0, &[0x34]);
    assert_eq!(pit.divisor(), 0x34);
}

#[test]
fn unknown_port_is_ignored() {
    let mut pit = Pit::new();
    let initial = pit.divisor();
    pit.output(0x7, &[0xFF]);
    assert_eq!(pit.divisor(), initial);
}

#[test]
fn input_returns_zeroes_of_requested_length() {
    let mut pit = Pit::new();
    assert_eq!(pit.input(0, 4), vec![0, 0, 0, 0]);
    assert_eq!(pit.input(0, 0), Vec::<u8>::new());
}

#[test]
fn tick_without_programming_or_irq_handler_does_nothing() {
    let mut pit = Pit::new();
    // new PIT is not programmed and has no handler: tick is a no-op
    pit.tick();
    assert_eq!(pit.acc_ns(), 0.0);
}
