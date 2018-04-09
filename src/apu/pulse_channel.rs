use super::{Sweep, SweepNegationMode};
use super::Envelope;
use super::LengthCounter;
use super::Sequencer;

pub const PULSE_WAVEFORMS: [[u8; 8]; 4] = [
    [0, 1, 0, 0, 0, 0, 0, 0],
    [0, 1, 1, 0, 0, 0, 0, 0],
    [0, 1, 1, 1, 1, 0, 0, 0],
    [1, 0, 0, 1, 1, 1, 1, 1],
];

pub struct PulseChannel {
    sweep: Sweep,
    envelope: Envelope,
    sequencer: Sequencer,
    length_counter: LengthCounter,
    duty_cycle: usize,
}

impl PulseChannel {
    pub fn new(sweep_negation_mode: SweepNegationMode) -> Self {
        PulseChannel {
            envelope: Envelope::new(),
            length_counter: LengthCounter::new(),
            sequencer: Sequencer::new(PULSE_WAVEFORMS[0].len()),
            sweep: Sweep::new(sweep_negation_mode),
            duty_cycle: 0,
        }
    }

    pub fn write_register(&mut self, address: u16, value: u8) {
        match address % 4 {
            0 => {
                self.duty_cycle = value as usize >> 6;
                self.envelope.write_register(value);
                self.length_counter.set_halted(value & 0b0010_0000 != 0)
            }
            1 => self.sweep.write_register(value),
            2 => self.sequencer.set_period_low(value),
            3 => {
                self.length_counter.write_register(value);
                self.sequencer.set_period_high(value & 0b111);
                self.envelope.start();
                self.sequencer.current_step = 0; // TODO really?
            }

            _ => panic!(),
        }
    }

    pub fn sample(&self) -> u8 {
        // TODO: removing the target period check makes arkanoid sound effects work
        if self.length_counter.active() && self.sequencer.period >= 8
            && self.sweep.target_period(&self.sequencer) < 0x800
        {
            PULSE_WAVEFORMS[self.duty_cycle][self.sequencer.current_step] * self.envelope.volume()
        } else {
            0
        }
    }

    pub fn tick_quarter_frame(&mut self) {
        self.envelope.tick();
    }

    pub fn tick_half_frame(&mut self) {
        self.length_counter.tick();
        self.sweep.tick(&mut self.sequencer);
    }

    pub fn tick_sequencer(&mut self) {
        self.sequencer.tick(true);
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
