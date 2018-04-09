use super::Sequencer;

#[derive(Copy, Clone, PartialEq)]
pub enum SweepNegationMode {
    OnesCompliment = 1,
    TwosCompliment = 0,
}

#[derive(Copy, Clone, PartialEq)]
pub struct Sweep {
    enabled: bool,
    reload: bool,
    shift: u8,
    negate: bool,
    negation_mode: SweepNegationMode,
    period: u8,
    counter: u8,
}

impl Sweep {
    pub fn new(negation_mode: SweepNegationMode) -> Self {
        Sweep {
            enabled: false,
            reload: false,
            shift: 0,
            negate: false,
            period: 0,
            counter: 0,
            negation_mode,
        }
    }

    pub fn write_register(&mut self, data: u8) {
        self.enabled = data & 0b1000_0000 != 0;
        self.period = (data & 0b0111_0000) >> 4;
        self.negate = data & 0b0000_1000 != 0;
        self.shift = data & 0b0000_0111;
        self.reload = true;
    }

    pub fn tick(&mut self, sequencer: &mut Sequencer) {
        if self.counter == 0 && self.enabled && self.shift > 0 && sequencer.period >= 8 {
            let new_period = self.target_period(sequencer);

            if new_period < 0x800 {
                sequencer.period = new_period;
                sequencer.counter = new_period;
            }
        }

        if self.counter == 0 || self.reload {
            self.counter = self.period;
            self.reload = false;
        } else {
            self.counter -= 1;
        }
    }

    pub fn target_period(&self, sequencer: &Sequencer) -> u16 {
        let period = sequencer.period;
        if self.negate {
            period - (period >> self.shift) - self.negation_mode as u16
        } else {
            period + (period >> self.shift)
        }
    }
}
