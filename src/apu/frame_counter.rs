#[derive(Debug, Copy, Clone, PartialEq)]
enum Mode {
    Zero,
    One,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum FrameResult {
    None,
    Quarter,
    Half,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct FrameCounter {
    pub counter: i64,
    pub cycles: u64,
    pub irq_enabled: bool,
    pub public_irq_flag: bool,
    pub private_irq_flag: bool,
    mode: Mode,
}

impl FrameCounter {
    pub fn new() -> Self {
        FrameCounter {
            counter: 0,
            cycles: 0,
            irq_enabled: true,
            public_irq_flag: false,
            private_irq_flag: false,
            mode: Mode::Zero,
        }
    }

    pub fn write_register(&mut self, value: u8, cycles: u64) -> FrameResult {
        self.irq_enabled = value & 0x40 == 0;
        if !self.irq_enabled {
            self.public_irq_flag = false;
            self.private_irq_flag = false;
        }

        self.mode = if value & 0x80 == 0 {
            Mode::Zero
        } else {
            Mode::One
        };

        self.counter = if cycles & 1 == 0 { 0 } else { -1 };

        match self.mode {
            Mode::Zero => FrameResult::None,
            Mode::One => FrameResult::Half,
        }
    }

    pub fn tick(&mut self) -> FrameResult {
        let result = match self.mode {
            Mode::Zero => self.tick_mode_zero(),
            Mode::One => self.tick_mode_one(),
        };
        self.counter += 1;
        result
    }

    fn tick_mode_zero(&mut self) -> FrameResult {
        match self.counter {
            7_459 => FrameResult::Quarter,
            14_915 => FrameResult::Half,
            22_373 => FrameResult::Quarter,
            29_830 => {
                self.trigger_irq();
                FrameResult::None
            }
            29_831 => {
                self.trigger_irq();
                self.publish_irq();
                FrameResult::Half
            }
            29_832 => {
                self.trigger_irq();
                self.publish_irq();
                // The counter *actually* rolls over to zero on cycle 29_830.
                // The actions at 29_831 and 29_832 happen after the rollover.
                // We emulate that by resetting our counter at 29_832 and skipping
                // it ahead as if it had been reset at 29_830.
                self.counter = 2;
                FrameResult::None
            }
            _ => FrameResult::None,
        }
    }

    fn tick_mode_one(&mut self) -> FrameResult {
        match self.counter {
            7_459 => FrameResult::Quarter,
            14_915 => FrameResult::Half,
            22_373 => FrameResult::Quarter,
            37_283 => {
                // The counter *actually* rolls over to zero on cycle 37_282.
                // The Half-frame signal is sent 1 tick after. We emulate this
                // behavior by adding an extra tick to the clock, then skipping
                // the clock ahead as if it had been reset at 37_282.
                self.counter = 1;
                FrameResult::Half
            }
            _ => FrameResult::None,
        }
    }

    pub fn trigger_irq(&mut self) {
        if self.irq_enabled {
            self.private_irq_flag = true;
        }
    }
    pub fn publish_irq(&mut self) {
        self.public_irq_flag = self.private_irq_flag;
    }
}
