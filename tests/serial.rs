use std::fs::File;

use ferrumvm::{device_maps::io::IODevice, devices::serial::{Serial, SerialMode}};

fn logfile_serial() -> Serial {
    // Use a temp file so construction doesn't touch the terminal.
    let path = std::env::temp_dir().join(format!(
        "ferrumvm-serial-test-{}.log",
        std::process::id()
    ));
    Serial::new(SerialMode::LogFile(
        File::options()
            .create(true)
            .truncate(true)
            .write(true)
            .open(&path)
            .unwrap(),
    ))
}

#[test]
fn scratch_register_read_write_roundtrip() {
    let mut s = logfile_serial();
    // SCR is port 7 (relative 7, since range starts at 0)
    s.output(7, &[0xAB]);
    assert_eq!(s.input(7, 1), vec![0xAB]);
}

#[test]
fn lsr_has_transmitter_ready_and_empty_bits() {
    let mut s = logfile_serial();
    let lsr = s.input(5, 1)[0];
    assert_ne!(lsr & 0x60, 0); // bits 5 (empty) and 6 (ready)
}

#[test]
fn reading_data_with_none_empty_returns_zeroes() {
    let mut s = logfile_serial();
    let data = s.input(0, 3);
    assert_eq!(data, vec![0, 0, 0]);
}

#[test]
fn enqueued_data_is_read_back_from_data_register() {
    let mut s = logfile_serial();
    s.set_data(vec![0x41, 0x42]);
    // data register returns bytes in LIFO order (pop_back)
    let first = s.input(0, 1);
    assert_eq!(first, vec![0x42]);
    let second = s.input(0, 1);
    assert_eq!(second, vec![0x41]);
}

#[test]
fn ier_masks_to_low_nibble() {
    let mut s = logfile_serial();
    s.output(1, &[0xFF]);
    let ier = s.input(1, 1)[0];
    assert_eq!(ier, 0x0F); // masked to 4 bits
}

#[test]
fn dlab_selects_divisor_latch_registers() {
    let mut s = logfile_serial();
    // enable DLAB via LCR bit 7
    s.output(3, &[0x80]);
    // DLL / DLH read/write while DLAB set
    s.output(0, &[0x34]); // DLL
    s.output(1, &[0x12]); // DLH
    assert_eq!(s.input(0, 1), vec![0x34]);
    assert_eq!(s.input(1, 1), vec![0x12]);
}

#[test]
fn writing_transmitter_marks_thr_empty() {
    let mut s = logfile_serial();
    // enable Transmit-Holding-Empty interrupt (IER bit 1)
    s.output(1, &[0x02]);
    s.output(0, &[0x48]); // 'H'
    // IIR bit0 = 0 => interrupt pending (transmit-holding-empty)
    let iir = s.input(2, 1)[0];
    assert_eq!(iir & 0x01, 0);
}
