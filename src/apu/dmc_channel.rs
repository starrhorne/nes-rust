use cartridge::Cartridge;
use std::cell::RefCell;
use std::rc::Rc;

#[cfg_attr(rustfmt, rustfmt_skip)]
pub const PERIODS: [u8; 16] = [
    214, 190, 170, 160, 143, 127, 113, 107, 95, 80, 71, 64, 53, 42, 36, 27
];

pub struct DmcChannel {
    cartridge: Option<Rc<RefCell<Cartridge>>>,
    pub irq_enabled: bool,
    pub irq_flag: bool,
    enabled: bool,
    output: u8,
    sample_address: u16,
    sample_length: u16,
    current_address: u16,
    current_length: u16,
    shift_register: u8,
    bit_count: u8,
    period: u8,
    counter: u8,
    looping: bool,
    cpu_stall_cycles: u8,
}

impl DmcChannel {
    pub fn new() -> Self {
        DmcChannel {
            cartridge: None,
            irq_enabled: false,
            irq_flag: false,
            enabled: false,
            output: 0,
            sample_address: 0,
            sample_length: 0,
            current_address: 0,
            current_length: 0,
            shift_register: 0,
            bit_count: 0,
            period: 0,
            counter: 0,
            looping: false,
            cpu_stall_cycles: 0,
        }
    }

    pub fn reset_cpu_stall_cycles(&mut self) -> u8 {
        let c = self.cpu_stall_cycles;
        self.cpu_stall_cycles = 0;
        c
    }

    pub fn sample(&self) -> u8 {
        self.output
    }

    pub fn write_register(&mut self, address: u16, value: u8) {
        match address {
            0x4010 => {
                self.irq_enabled = value & 0b1000_0000 != 0;
                self.irq_flag &= self.irq_enabled;
                self.looping = value & 0b0100_0000 != 0;
                self.period = PERIODS[value as usize & 0x0F];
            }
            0x4011 => {
                self.output = value & 0b0111_1111;
            }
            0x4012 => {
                self.sample_address = 0xC000 + (value as u16 * 64);
            }
            0x4013 => {
                self.sample_length = 1 + (value as u16 * 16);
            }
            _ => panic!(),
        }
    }

    pub fn set_enabled(&mut self, value: bool) {
        self.irq_flag = false;
        self.enabled = value;

        if !self.enabled {
            self.current_length = 0;
        } else {
            if self.current_length == 0 {
                self.restart();
            }
        }
    }

    pub fn set_cartridge(&mut self, cartridge: Rc<RefCell<Cartridge>>) {
        self.cartridge = Some(cartridge);
    }

    pub fn restart(&mut self) {
        self.current_address = self.sample_address;
        self.current_length = self.sample_length;
    }

    pub fn tick_sequencer(&mut self) {
        if self.enabled {
            self.tick_read();
            self.tick_shift();
        }
    }

    fn tick_read(&mut self) {
        if self.current_length > 0 && self.bit_count == 0 {
            self.cpu_stall_cycles += 4;
            let a = self.current_address;
            self.shift_register = match self.cartridge {
                Some(ref c) => c.borrow_mut().read_prg_byte(a),
                None => 0,
            };
            self.bit_count = 8;
            self.current_address = self.current_address.wrapping_add(1);
            if self.current_address == 0 {
                self.current_address = 0x8000;
            }
            self.current_length -= 1;
            if self.current_length == 0 && self.looping {
                self.restart();
            } else if self.current_length == 0 && self.irq_enabled {
                self.irq_flag = true;
            }
        }
    }

    fn tick_shift(&mut self) {
        if self.counter == 0 {
            self.counter = self.period - 1;
            if self.bit_count > 0 {
                if self.shift_register & 1 == 1 {
                    if self.output <= 125 {
                        self.output += 2;
                    }
                } else {
                    if self.output >= 2 {
                        self.output -= 2;
                    }
                }
                self.shift_register >>= 1;
                self.bit_count -= 1;
            }
        } else {
            self.counter -= 1;
        }
    }

    pub fn playing(&self) -> bool {
        self.current_length > 0
    }
}
