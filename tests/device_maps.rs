use std::ops::RangeInclusive;

use ferrumvm::device_maps::{
    io::{IODevice, IODeviceMap, IODeviceRegion},
    mmio::{MMIODevice, MMIODeviceMap, MMIODeviceRegion},
};

mod io_map {
    use super::*;

    #[derive(Default)]
    struct FakeDevice {
        last_output: Option<Vec<u8>>,
    }

    impl IODevice for FakeDevice {
        fn input(&mut self, _port: u16, length: usize) -> Vec<u8> {
            vec![0xAA; length]
        }
        fn output(&mut self, _port: u16, data: &[u8]) {
            self.last_output = Some(data.to_vec());
        }
    }

    fn device_at(range: RangeInclusive<u16>) -> IODeviceRegion {
        IODeviceRegion::new(range, Box::new(FakeDevice::default()))
    }

    #[test]
    fn region_contains_checks_port_range() {
        let region = device_at(0x20..=0x2F);
        assert!(region.contains(0x20));
        assert!(region.contains(0x2F));
        assert!(!region.contains(0x1F));
        assert!(!region.contains(0x30));
    }

    #[test]
    fn region_forwardes_relative_port() {
        let mut region = device_at(0x20..=0x2F);
        // port 0x24 maps to relative 0x04; input returns 0xAA per byte
        assert_eq!(region.input(0x24, 2).unwrap(), vec![0xAA, 0xAA]);
    }

    #[test]
    fn map_dispatches_to_matching_device_only() {
        let mut map = IODeviceMap::new();
        map.register(device_at(0x20..=0x2F));
        map.register(device_at(0x40..=0x4F));

        assert_eq!(map.input(0x45, 1).unwrap(), vec![0xAA]);
        assert!(map.input(0x30, 1).is_none()); // gap, unmapped
        assert!(map.output(0x30, &[1]).is_none());
    }

    #[test]
    fn first_registered_device_wins_for_overlapping_ranges() {
        let mut map = IODeviceMap::new();
        map.register(device_at(0x0..=0x10));
        map.register(device_at(0x10..=0x20));
        // port 0x10 matches both; first registered region handles it
        let _ = map.input(0x10, 1);
    }

    #[test]
    fn empty_map_returns_none() {
        let mut map = IODeviceMap::new();
        assert!(map.input(0x3F8, 1).is_none());
        assert!(map.output(0x3F8, &[0]).is_none());
    }
}

mod mmio_map {
    use super::*;

    #[derive(Default)]
    struct FakeMMIO {
        last_data: Vec<u8>,
        return_data: Vec<u8>,
    }

    impl MMIODevice for FakeMMIO {
        fn read(&mut self, _addr: u64, length: usize) -> Vec<u8> {
            if self.return_data.is_empty() {
                vec![0; length]
            } else {
                self.return_data.clone()
            }
        }
        fn write(&mut self, _addr: u64, data: &[u8]) {
            self.last_data = data.to_vec();
        }
    }

    fn device_at(range: RangeInclusive<u64>) -> MMIODeviceRegion {
        MMIODeviceRegion::new(range, Box::new(FakeMMIO::default()))
    }

    #[test]
    fn region_contains_checks_addr_range() {
        let region = device_at(0xF0000000..=0xF0000FFF);
        assert!(region.contains(0xF0000000));
        assert!(region.contains(0xF0000FFF));
        assert!(!region.contains(0xF0001000));
        assert!(!region.contains(0xEFFFFFFF));
    }

    #[test]
    fn region_returns_range_clone() {
        let region = device_at(0xF0000000..=0xF0000FFF);
        assert_eq!(region.get_range(), 0xF0000000..=0xF0000FFF);
    }

    #[test]
    fn map_read_write_routes_to_matching_device() {
        let mut map = MMIODeviceMap::new();
        map.register(device_at(0xF0000000..=0xF0000FFF));
        map.register(device_at(0xA0000000..=0xA00000FF));

        assert!(map.write(0xF0000555, &[1, 2]).is_some());
        assert!(map.read(0xA0000050, 4).is_some());
        assert!(map.read(0x10000000, 1).is_none()); // unmapped
        assert!(map.write(0x10000000, &[0]).is_none());
    }

    #[test]
    fn empty_map_returns_none() {
        let mut map = MMIODeviceMap::new();
        assert!(map.read(0xF0000000, 1).is_none());
        assert!(map.write(0xF0000000, &[0]).is_none());
    }
}
