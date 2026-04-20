use chrono::{Datelike, Timelike, Utc};

use crate::device_maps::io::IODevice;

#[derive(Debug, Clone)]
enum CmosRegister {
    Unset,
    Seconds,
    Minute,
    Hour,
    SecondAlarm,
    MinuteAlarm,
    HourAlarm,
    DayOfWeek,
    DayOfMonth,
    Month,
    Year,
    StatusA,
    StatusB,
    StatusC,
    StatusD,
}

pub struct Cmos {
    reg: CmosRegister,

    uip: bool,
    oscillator: u8,
    rate: u8,

    halt: bool,
    periodic_irq: bool,
    enable_alrm: bool,
    enable_up_ended_irq: bool,
    squ_wave: bool,
    binary: bool,
    hour_12: bool,
    ds_enable: bool
}

fn to_bcd(val: u8) -> u8 {
    ((val / 10) << 4) | (val % 10)
}

impl Cmos {
    pub fn new() -> Self {
        Self {
            reg: CmosRegister::Unset,
            uip: false,
            oscillator: 2,
            rate: 6,
            halt: false,
            periodic_irq: false,
            enable_alrm: false,
            enable_up_ended_irq: false,
            squ_wave: false,
            binary: false,
            hour_12: false,
            ds_enable: false,
        }
    }
}

// IO device mapped 0x70 - 0x71
impl IODevice for Cmos {
    fn input(&mut self, port: u16, length: usize) -> Vec<u8> {
        match port {
            1 => match self.reg {
                CmosRegister::Seconds => {
                    let val = Utc::now().second() as u8;
                    let val = if self.binary { val } else { to_bcd(val) };
                    vec![val; length]
                }
                CmosRegister::Minute => {
                    let val = Utc::now().minute() as u8;
                    let val = if self.binary { val } else { to_bcd(val) };
                    vec![val; length]
                }
                CmosRegister::Hour => {
                    let val = if self.hour_12 {Utc::now().hour12().1 as u8} else {Utc::now().hour() as u8};
                    let val = if self.binary { val } else { to_bcd(val) };
                    vec![val; length]
                }
                CmosRegister::DayOfWeek => {
                    let now = Utc::now();
                    vec![now.weekday().number_from_monday() as u8; length]
                }
                CmosRegister::DayOfMonth => {
                    let now = Utc::now();
                    vec![now.day() as u8; length]
                }
                CmosRegister::Month => {
                    let now = Utc::now();
                    vec![now.month() as u8; length]
                }
                CmosRegister::Year => {
                    let now = Utc::now();
                    vec![(now.year() % 100) as u8; length]
                }
                CmosRegister::StatusA => {
                    let a: u8 = ((self.uip as u8) << 7)
                        | (self.oscillator << 4)
                        | self.rate;
                    vec![a; length]
                }
                CmosRegister::StatusB => {
                    let b: u8 = ((self.halt as u8) << 7)
                        | ((self.periodic_irq as u8) << 6)
                        | ((self.enable_alrm as u8) << 5)
                        | ((self.enable_up_ended_irq as u8) << 4)
                        | ((self.squ_wave as u8) << 3)
                        | ((self.binary as u8) << 2)
                        | ((!self.hour_12 as u8) << 1)  // bit1: 1=24hr, 0=12hr
                        | (self.ds_enable as u8);
                    vec![b; length]
                }
                CmosRegister::StatusC => vec![0; length],  // no IRQ support yet
                CmosRegister::StatusD => vec![0x80; length], // bit7 = battery good
                _ => vec![0; length],
            },
            _ => vec![0; length],
        }
    }

    fn output(&mut self, port: u16, data: &[u8]) {
        match port {
            0 => {
                let index = data[data.len() - 1] & 0x7F;
                self.reg = match index {
                    0x00 => CmosRegister::Seconds,
                    0x01 => CmosRegister::SecondAlarm,
                    0x02 => CmosRegister::Minute,
                    0x03 => CmosRegister::MinuteAlarm,
                    0x04 => CmosRegister::Hour,
                    0x05 => CmosRegister::HourAlarm,
                    0x06 => CmosRegister::DayOfWeek,
                    0x07 => CmosRegister::DayOfMonth,
                    0x08 => CmosRegister::Month,
                    0x09 => CmosRegister::Year,
                    0x0A => CmosRegister::StatusA,
                    0x0B => CmosRegister::StatusB,
                    0x0C => CmosRegister::StatusC,
                    0x0D => CmosRegister::StatusD,
                    _ => self.reg.clone(),
                }
            }
            1 => {
                if let CmosRegister::StatusB = self.reg {
                    let val = data[0];
                    self.halt               = (val >> 7) & 1 != 0;
                    self.periodic_irq       = (val >> 6) & 1 != 0;
                    self.enable_alrm        = (val >> 5) & 1 != 0;
                    self.enable_up_ended_irq= (val >> 4) & 1 != 0;
                    self.squ_wave           = (val >> 3) & 1 != 0;
                    self.binary             = (val >> 2) & 1 != 0;
                    self.hour_12            = (val >> 1) & 1 == 0; // 0=12hr, 1=24hr
                    self.ds_enable          = val & 1 != 0;
                }
            },
            _ => {}
        }
    }
}
