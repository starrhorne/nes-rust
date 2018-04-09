#[cfg_attr(rustfmt, rustfmt_skip)]
pub const LENGTHS: [u8; 32] = [
    0x0a, 0xfe, 0x14, 0x02, 0x28, 0x04, 0x50, 0x06, 0xa0, 0x08, 0x3c, 0x0a, 0x0e, 0x0c, 0x1a,
    0x0e, 0x0c, 0x10, 0x18, 0x12, 0x30, 0x14, 0x60, 0x16, 0xc0, 0x18, 0x48, 0x1a, 0x10, 0x1c, 0x20, 0x1e,
];

pub struct LengthCounter {
    counter: u8,
    pub enabled: bool,
    halted: bool,
    pending_halted: Option<bool>,
    pending_register: Option<u8>,
}

impl LengthCounter {
    pub fn new() -> Self {
        LengthCounter {
            counter: 0,
            enabled: false,
            halted: false,
            pending_halted: None,
            pending_register: None,
        }
    }

    pub fn write_register(&mut self, value: u8) {
        self.pending_register = Some(value);
    }

    pub fn set_halted(&mut self, value: bool) {
        self.pending_halted = Some(value);
    }

    pub fn set_enabled(&mut self, v: bool) {
        self.enabled = v;
        if !v {
            self.counter = 0;
        }
    }

    pub fn update_pending(&mut self) {
        if let Some(v) = self.pending_halted {
            self.halted = v;
            self.pending_halted = None;
        }

        if let Some(value) = self.pending_register {
            if self.enabled {
                self.counter = LENGTHS[(value >> 3) as usize];
            }
            self.pending_register = None;
        }
    }

    pub fn tick(&mut self) {
        if let Some(_) = self.pending_register {
            if self.counter == 0 {
                return;
            } else {
                self.pending_register = None;
            }
        }
        if self.enabled && !self.halted && self.counter > 0 {
            self.counter -= 1;
        }
    }

    pub fn active(&self) -> bool {
        self.enabled && self.counter > 0
    }

    pub fn playing(&self) -> bool {
        self.counter > 0
    }
}
