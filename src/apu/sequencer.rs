pub struct Sequencer {
    pub counter: u16,
    pub period: u16,
    steps: usize,
    pub current_step: usize,
}

impl Sequencer {
    pub fn new(steps: usize) -> Self {
        Sequencer {
            counter: 0,
            period: 0,
            current_step: 0,
            steps,
        }
    }

    pub fn tick(&mut self, step_enabled: bool) -> bool {
        if self.counter == 0 {
            self.counter = self.period;
            if step_enabled {
                self.current_step = (self.current_step + 1) % self.steps;
            }
            true
        } else {
            self.counter -= 1;
            false
        }
    }

    pub fn set_period_low(&mut self, value: u8) {
        self.period = (self.period & 0xFF00) | value as u16;
    }

    pub fn set_period_high(&mut self, value: u8) {
        self.period = (self.period & 0x00FF) | ((value as u16 & 0b111) << 8);
    }
}
