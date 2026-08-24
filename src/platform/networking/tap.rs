use std::{
    collections::VecDeque,
    fs,
    net::{Ipv4Addr, Ipv6Addr},
};
use tappers::AddAddressV4;
use tappers::{Interface, Tap};

use crate::platform::networking::firewall::setup_ferrumvm_firewall;

const IFACE_NAME: &str = "ferrum-tap0";

pub struct TAPDevice {
    tap: Tap,
    pub packet_recive_queue: VecDeque<Vec<u8>>,
}

impl TAPDevice {
    pub fn new() -> Result<Self, String> {
        if !Self::ensure_ip_forwarding() {
            eprint!("Could not enable ip forwading\r\n");
        }
        let fw_res = setup_ferrumvm_firewall(IFACE_NAME);
        if fw_res.is_err() {
            eprint!("Could not setup firewall rules\r\n");
        };

        let Ok(tap_name) = Interface::new(IFACE_NAME) else {
            return Err("Could not name the Tap device".to_string());
        };

        let Ok(mut tap) = Tap::new_named(tap_name) else {
            return Err("Tap Could not be created".to_string());
        };

        let Ok(_) = tap.add_addr(Ipv6Addr::new(0, 0, 0, 0, 0, 0xffff, 0xc00a, 0x2ff)) else {
            return Err("Could not add an IP Address".to_string());
        };
        let mut addr_req = AddAddressV4::new(Ipv4Addr::new(10, 0, 0, 1));
        addr_req.set_netmask(24);
        let Ok(_) = tap.add_addr(addr_req) else {
            return Err("Could not add an IPv4 address".to_string());
        };

        let Ok(_) = tap.set_nonblocking(true) else {
            return Err("Could not enable non-blocking on the device".to_string());
        };

        let Ok(_) = tap.set_up() else {
            return Err("Could not enable the device".to_string());
        };

        Ok(Self {
            tap,
            packet_recive_queue: VecDeque::new(),
        })
    }

    fn ensure_ip_forwarding() -> bool {
        let Ok(_) = fs::write("/proc/sys/net/ipv4/ip_forward", "1") else {
            return false;
        };
        true
    }

    pub fn get_next_packet(&mut self) -> Option<Vec<u8>> {
        self.packet_recive_queue.pop_front()
    }

    pub fn add_packet_font(&mut self, packet: Vec<u8>) {
        self.packet_recive_queue.push_front(packet);
    }

    pub fn send_packet(&mut self, packet: Vec<u8>) -> Result<(), String> {
        let Ok(_) = self.tap.send(packet.iter().as_slice()) else {
            return Err("Could not send packet!".to_string());
        };
        Ok(())
    }

    pub fn update(&mut self) {
        let mut recv_buf = [0u8; 65536];
        match self.tap.recv(&mut recv_buf) {
            Ok(amount) => {
                let packet = recv_buf[0..amount].to_vec();
                self.packet_recive_queue.push_back(packet);
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                // no packet ready, nothing to do
            }
            Err(_) => {
                // real error — decide how you want to handle/log this
            }
        }
    }
}
