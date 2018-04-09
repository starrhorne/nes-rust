use super::address::Address;
use super::control::Control;
use super::mask::Mask;
use super::status::Status;
use super::vram::Vram;

use rand::{thread_rng, Rng};

pub struct Registers {
    pub vram: Vram,
    pub t_address: Address,
    pub v_address: Address,
    pub fine_x: u8,
    pub oam_ram: [u8; 0x100],
    oam_address: u8,
    pub control: Control,
    pub mask: Mask,
    pub status: Status,
    latch: bool,
    open_bus: u8,
    pub force_nmi: bool,
    pub vblank_suppress: bool,
}

impl Registers {
    pub fn new() -> Self {
        let mut p = Registers {
            vram: Vram::new(),
            v_address: Address(0),
            t_address: Address(0),
            fine_x: 0,
            oam_ram: [0; 0x100],
            oam_address: 0,
            control: Control(0),
            mask: Mask(0),
            status: Status(0),
            latch: false,
            open_bus: 0,
            force_nmi: false,
            vblank_suppress: false,
        };
        p.reset();
        p
    }

    pub fn reset(&mut self) {
        self.control = Control(0);
        self.status = Status(0);
        self.mask = Mask(0);
        self.oam_ram = [0; 0x100];
        self.vram.reset();
    }

    pub fn write_register(&mut self, address: u16, value: u8) {
        self.open_bus = value;
        match address % 8 {
            0 => self.write_control(value),
            1 => self.write_mask(value),
            2 => (),
            3 => self.write_oam_address(value),
            4 => self.write_oam_data(value),
            5 => self.write_scroll(value),
            6 => self.write_address(value),
            7 => self.write_data(value),
            _ => panic!("Invalid PPU register {:X}", address),
        }
    }

    pub fn read_register(&mut self, address: u16) -> u8 {
        let result = match address % 8 {
            0 => self.open_bus,
            1 => self.open_bus,
            2 => self.read_status() | (self.open_bus & 0b11111),
            3 => self.open_bus,
            4 => self.read_oam_data(),
            5 => self.open_bus,
            6 => self.open_bus,
            7 => {
                if let 0x3f00...0x3fff = self.v_address.address() {
                    self.read_data() | (self.open_bus & 0b1100_0000)
                } else {
                    self.read_data()
                }
            }
            _ => panic!("Invalid PPU register {:X}", address),
        };
        self.open_bus = result;
        result
    }

    pub fn tick_decay(&mut self) {
        let mut rng = thread_rng();
        for i in 0..8 {
            if rng.gen_weighted_bool(4) {
                self.open_bus &= !(1 << i);
            }
        }
    }

    fn write_control(&mut self, value: u8) {
        let control = Control(value);
        if !self.control.nmi_on_vblank() && control.nmi_on_vblank() {
            self.force_nmi = true;
        }
        self.control = control;
        self.t_address.set_nametable(self.control.nametable());
    }

    fn write_mask(&mut self, value: u8) {
        self.mask = Mask(value);
    }

    fn write_oam_address(&mut self, value: u8) {
        self.oam_address = value;
    }

    pub fn write_oam_data(&mut self, value: u8) {
        self.oam_ram[self.oam_address as usize] = value;
        self.oam_address = self.oam_address.wrapping_add(1);
    }

    fn read_oam_data(&mut self) -> u8 {
        if self.oam_address % 4 == 2 {
            self.oam_ram[self.oam_address as usize] & 0b1110_0011
        } else {
            self.oam_ram[self.oam_address as usize]
        }
    }

    fn write_scroll(&mut self, value: u8) {
        if self.latch {
            self.t_address.set_fine_y(value);
            self.t_address.set_coarse_y(value >> 3);
        } else {
            self.fine_x = value & 0b0000_0111;
            self.t_address.set_coarse_x(value >> 3);
        }
        self.latch = !self.latch
    }

    fn write_address(&mut self, value: u8) {
        if self.latch {
            self.t_address.set_low_byte(value);
            self.v_address = self.t_address.clone();
        } else {
            self.t_address.set_high_byte(value);
        }
        self.latch = !self.latch
    }

    fn write_data(&mut self, value: u8) {
        self.vram.write_byte(self.v_address.address(), value);
        self.v_address.increment(self.control.increment_amount());
    }

    fn read_status(&mut self) -> u8 {
        let result = self.status.get();
        self.status.set_vblank(false);
        self.latch = false;
        self.vblank_suppress = true;
        result
    }

    fn read_data(&mut self) -> u8 {
        let address = self.v_address.address();
        self.v_address.increment(self.control.increment_amount());
        self.vram.buffered_read_byte(address)
    }
}

#[cfg(test)]
mod test {

    use super::*;

    #[test]
    fn test_write_control() {
        let mut reg = Registers::new();
        reg.write_register(0x2000, 0b1010_1010);
        assert_eq!(reg.control.0, 0b1010_1010);
    }

    #[test]
    fn test_write_mask() {
        let mut reg = Registers::new();
        reg.write_register(0x2001, 0b1010_1010);
        assert_eq!(reg.mask.0, 0b1010_1010);
    }

    #[test]
    fn test_write_oam_address() {
        let mut reg = Registers::new();
        reg.write_register(0x2003, 0xF0);
        assert_eq!(reg.oam_address, 0xF0);
    }

    #[test]
    fn test_write_oam_data() {
        let mut reg = Registers::new();
        reg.oam_address = 5;
        reg.write_register(0x2004, 0xF0);
        assert_eq!(reg.oam_ram[5], 0xF0);
        assert_eq!(reg.oam_address, 6);
    }

    #[test]
    fn test_write_scroll() {
        let mut reg = Registers::new();
        reg.write_register(0x2005, 0b10101_010);
        assert_eq!(reg.fine_x, 0b010);
        assert_eq!(reg.t_address.coarse_x(), 0b10101);
        assert_eq!(reg.latch, true);

        reg.write_register(0x2005, 0b01010_101);
        assert_eq!(reg.t_address.fine_y(), 0b101);
        assert_eq!(reg.t_address.coarse_y(), 0b01010);
        assert_eq!(reg.latch, false);
    }

    #[test]
    fn test_write_address() {
        let mut reg = Registers::new();
        reg.write_register(0x2006, 0b11_101010);
        assert_eq!(reg.t_address.high_byte(), 0b00_101010);
        assert_ne!(reg.t_address, reg.v_address);
        assert_eq!(reg.latch, true);

        reg.write_register(0x2006, 0b1010_1010);
        assert_eq!(reg.t_address.0, 0b0010_1010_1010_1010);
        assert_eq!(reg.t_address, reg.v_address);
        assert_eq!(reg.latch, false);
    }

    #[test]
    fn test_write_data() {
        let mut reg = Registers::new();
        reg.v_address.0 = 0x2000;
        reg.write_register(0x2007, 0xF0);
        assert_eq!(reg.vram.read_byte(0x2000), 0xF0);
        assert_eq!(reg.v_address.0, 0x2001);

        reg.control.0 = 0b0000_0100; // vertical increment
        reg.write_register(0x2007, 0x0F);
        assert_eq!(reg.vram.read_byte(0x2001), 0x0F);
        assert_eq!(reg.v_address.0, 0x2001 + 32);
    }

    #[test]
    fn test_read_status() {
        let mut reg = Registers::new();
        reg.latch = true;
        reg.status.0 = 0b1110_0000;
        let r = reg.read_register(0x2002);
        assert_eq!(r, 0b1110_0000);
        assert_eq!(reg.latch, false);
        assert_eq!(reg.status.vblank(), false);
    }

    #[test]
    fn test_read_ghost_bits() {
        let mut reg = Registers::new();
        reg.write_register(0x2002, 0b1111_1111);
        reg.status.0 = 0;
        assert_eq!(reg.read_register(0x2002), 0b0001_1111);
        assert_eq!(reg.read_register(0x2000), 0b0001_1111);
        assert_eq!(reg.read_register(0x2001), 0b0001_1111);
        assert_eq!(reg.read_register(0x2003), 0b0001_1111);
        assert_eq!(reg.read_register(0x2005), 0b0001_1111);
        assert_eq!(reg.read_register(0x2006), 0b0001_1111);
    }
    #[test]
    fn test_read_oam_data() {
        let mut reg = Registers::new();
        reg.oam_ram[5] = 0x0F;
        reg.oam_address = 5;
        assert_eq!(reg.read_register(0x2004), 0x0F);
        assert_eq!(reg.oam_address, 5);
    }

    #[test]
    fn test_read_data_delayed() {
        let mut reg = Registers::new();
        reg.vram.write_byte(0x2001, 1);
        reg.vram.write_byte(0x2002, 2);
        reg.vram.write_byte(0x2003, 3);
        reg.v_address.0 = 0x2001;
        reg.read_register(0x2007);
        assert_eq!(reg.read_register(0x2007), 1);
        assert_eq!(reg.read_register(0x2007), 2);
        assert_eq!(reg.read_register(0x2007), 3);
    }

}
