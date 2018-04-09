use super::LengthCounter;
use super::Sequencer;

#[cfg_attr(rustfmt, rustfmt_skip)]
pub const TRIANGLE_WAVEFORM: [u8; 32] = [
    15, 14, 13, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1, 0,
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15,
];

pub struct TriangleChannel {
    length_counter: LengthCounter,
    sequencer: Sequencer,
    linear_counter: u8,
    linear_counter_start: bool,
    linear_counter_period: u8,
    control_flag: bool,
}

impl TriangleChannel {
    pub fn new() -> Self {
        TriangleChannel {
            length_counter: LengthCounter::new(),
            sequencer: Sequencer::new(TRIANGLE_WAVEFORM.len()),
            linear_counter: 0,
            control_flag: false,
            linear_counter_period: 0,
            linear_counter_start: false,
        }
    }

    pub fn write_register(&mut self, address: u16, value: u8) {
        match address {
            0x4008 => {
                self.control_flag = value & 0b1000_0000 != 0;
                self.length_counter.set_halted(value & 0b1000_0000 != 0);
                self.linear_counter_period = value & 0b0111_1111;
            }
            0x4009 => (),
            0x400A => {
                self.sequencer.set_period_low(value);
            }
            0x400B => {
                self.length_counter.write_register(value);
                self.sequencer.set_period_high(value & 0b111);
                self.linear_counter_start = true;
            }

            _ => panic!(),
        }
    }

    pub fn sample(&self) -> u8 {
        if self.active() && self.sequencer.period > 2 {
            TRIANGLE_WAVEFORM[self.sequencer.current_step]
        } else {
            0
        }
    }

    pub fn tick_sequencer(&mut self) {
        let sequencer_active = self.active();
        self.sequencer.tick(sequencer_active);
    }

    pub fn tick_quarter_frame(&mut self) {
        // Tick the linear counter
        if self.linear_counter_start {
            self.linear_counter = self.linear_counter_period;
        } else if self.linear_counter > 0 {
            self.linear_counter -= 1;
        }

        if !self.control_flag {
            self.linear_counter_start = false;
        }
    }

    pub fn tick_half_frame(&mut self) {
        self.length_counter.tick();
    }

    fn active(&self) -> bool {
        self.length_counter.active() && self.linear_counter > 0
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
