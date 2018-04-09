use super::Envelope;
use super::LengthCounter;

#[cfg_attr(rustfmt, rustfmt_skip)]
const PERIODS: [u16; 16] = [
    4, 8, 16, 32, 64, 96, 128, 160, 202, 254, 380, 508, 762, 1016, 2034, 4068,
];

pub struct NoiseChannel {
    envelope: Envelope,
    length_counter: LengthCounter,
    mode: bool,
    period: u16,
    counter: u16,
    shift: u16,
}

impl NoiseChannel {
    pub fn new() -> Self {
        NoiseChannel {
            envelope: Envelope::new(),
            length_counter: LengthCounter::new(),
            mode: false,
            period: 0,
            counter: 0,
            shift: 1,
        }
    }

    pub fn write_register(&mut self, address: u16, value: u8) {
        match address {
            0x400C => {
                self.length_counter.set_halted(value & 0b0010_0000 != 0);
                self.envelope.write_register(value);
            }
            0x400D => (),
            0x400E => {
                self.mode = value & 0b1000_0000 != 0;
                self.period = PERIODS[value as usize & 0b1111];
            }
            0x400F => {
                self.length_counter.write_register(value);
                self.envelope.start();
            }

            _ => panic!("bad noise register {:04X}", address),
        }
    }

    pub fn sample(&self) -> u8 {
        if self.length_counter.active() && self.shift & 1 == 0 {
            self.envelope.volume()
        } else {
            0
        }
    }

    pub fn tick_sequencer(&mut self) {
        if self.counter > 0 {
            self.counter -= 1;
        } else {
            self.counter = self.period;
            let bit1 = (self.shift >> (if self.mode { 6 } else { 1 })) & 1;
            let bit2 = self.shift & 1;
            self.shift = (self.shift >> 1) | (bit1 ^ bit2) << 14
        }
    }

    pub fn tick_quarter_frame(&mut self) {
        self.envelope.tick();
    }

    pub fn tick_half_frame(&mut self) {
        self.length_counter.tick();
    }

    pub fn playing(&mut self) -> bool {
        self.length_counter.playing()
    }

    pub fn set_enabled(&mut self, value: bool) {
        self.length_counter.set_enabled(value);
    }

    pub fn update_pending_length_counter(&mut self) {
        self.length_counter.update_pending();
    }
}
