#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Button {
    A = 0b0000_0001,
    B = 0b0000_0010,
    Select = 0b0000_0100,
    Start = 0b0000_1000,
    Up = 0b0001_0000,
    Down = 0b0010_0000,
    Left = 0b0100_0000,
    Right = 0b1000_0000,
}

pub struct Controller {
    button_states: u8,
    strobe: bool,
    cursor: usize,
}

impl Controller {
    pub fn new() -> Self {
        Controller {
            button_states: 0,
            strobe: false,
            cursor: 0,
        }
    }

    pub fn write_register(&mut self, value: u8) {
        self.strobe = value & 1 != 0;
        if self.strobe {
            self.cursor = 0;
        }
    }

    pub fn read_register(&mut self) -> u8 {
        let v = if self.cursor < 8 {
            self.button_states >> self.cursor & 1
        } else {
            1
        };

        if !self.strobe {
            self.cursor += 1;
        }

        0x40 | v
    }

    pub fn set_button_state(&mut self, button: Button, pressed: bool) {
        self.button_states &= !(button as u8);
        if pressed {
            self.button_states |= button as u8;
        }
    }
}
