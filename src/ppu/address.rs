bitfield!{
    #[derive(Copy, Clone, PartialEq)]
    pub struct Address(u16);
    impl Debug;
    pub u8,  coarse_x,   set_coarse_x:    4,  0;
    pub u8,  coarse_y,   set_coarse_y:    9,  5;
    pub u8,  nametable,  set_nametable:  11, 10;
    pub u8,  fine_y,     set_fine_y:     14, 12;
    pub u16, address,    _:              13,  0; // Full 14-bit address from PPUADDR
    pub u8,  high_byte,  set_high_byte:  13,  8; // High 6 bits of PPUADDR address
    pub u8,  low_byte,   set_low_byte:    7,  0; // Low 7 bits of PPUADDR address
    pub u16, get,        _:              14,  0; // Full data
}

impl Address {
    pub fn increment(&mut self, amount: u16) {
        self.0 = self.0.wrapping_add(amount);
    }

    pub fn nametable_address(&self) -> u16 {
        // Removes fine-y from address, since is used for inner-tile scrolling
        0x2000 | (self.get() & 0xFFF)
    }

    pub fn attribute_address(&self) -> u16 {
        let nt = self.nametable() as u16;
        let cy = self.coarse_y() as u16;
        let cx = self.coarse_x() as u16;
        // First find the base address of the attribute table then offset
        // by the x and y tiles. Each Attribute table entry covers 4 tiles.
        0x23C0 | (nt << 10) | ((cy / 4) << 3) | (cx / 4)
    }

    pub fn tile_offset(&self, tile_number: u8) -> u16 {
        (16 * tile_number as u16) | self.fine_y() as u16
    }

    // Copy coarse x and x-nametable bits
    pub fn copy_x(&mut self, other: Address) {
        self.0 = (self.0 & !0x041F) | (other.0 & 0x041F);
    }

    // Copy fine y, coarse y and y-nametable bits
    pub fn copy_y(&mut self, other: Address) {
        self.0 = (self.0 & !0x7BE0) | (other.0 & 0x7BE0);
    }

    pub fn scroll_x(&mut self) {
        if self.coarse_x() == 31 {
            self.set_coarse_x(0);
            self.0 ^= 0x0400 // switch horizontal nametable
        } else {
            let cx = self.coarse_x();
            self.set_coarse_x(cx + 1);
        }
    }

    pub fn scroll_y(&mut self) {
        let fy = self.fine_y();
        if fy < 7 {
            self.set_fine_y(fy + 1);
        } else {
            self.set_fine_y(0);
            let cy = self.coarse_y();
            if cy == 29 {
                // The last row of tiles
                self.set_coarse_y(0);
                self.0 ^= 0x0800; // Switch vertical nametable
            } else if cy == 31 {
                // Values 29-31 are out-of-bounds but used by some games
                // To implement a weird reverse scroll.
                self.set_coarse_y(0);
            } else {
                self.set_coarse_y(cy + 1);
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_loopy() {
        let a = Address(0b0_101_01_01010_10101);
        assert_eq!(a.coarse_x(), 0b10101);
        assert_eq!(a.coarse_y(), 0b01010);
        assert_eq!(a.nametable(), 0b01);
        assert_eq!(a.fine_y(), 0b101);
    }

    #[test]
    fn test_register_address() {
        let a = Address(0b1111_1111_1111_1111);
        assert_eq!(a.address(), 0b0011_1111_1111_1111);
        assert_eq!(a.low_byte(), 0b1111_1111);
        assert_eq!(a.high_byte(), 0b0011_1111);
    }

    #[test]
    fn test_nametable_address() {
        let a = Address(0b0101_1111_1111_1111);
        assert_eq!(a.nametable_address(), 0b0010_1111_1111_1111);
    }

    #[test]
    fn test_attribute_address() {
        let a = Address(0b0_101_01_01010_10101);
        assert_eq!(a.attribute_address(), 0b0010_01_1111_010_101);
    }

    #[test]
    fn test_tile_offset() {
        let a = Address(0b0_101_01_01010_10101);
        assert_eq!(a.tile_offset(0b111), 0b111_0_101);
    }

    #[test]
    fn test_copy_x() {
        let mut a = Address(0);
        let b = Address(0b1111_1111_1111_1111);
        a.copy_x(b);
        assert_eq!(a.0, 0b0_000_01_00000_11111);
    }

    #[test]
    fn test_copy_y() {
        let mut a = Address(0);
        let b = Address(0b1111_1111_1111_1111);
        a.copy_y(b);
        assert_eq!(a.0, 0b0_111_10_11111_00000);
    }

    #[test]
    fn test_scroll_x() {
        let mut a = Address(0);
        a.scroll_x();
        assert_eq!(a.coarse_x(), 1);

        a.set_coarse_x(31);
        a.scroll_x();
        assert_eq!(a.coarse_x(), 0);
        assert_eq!(a.nametable(), 1);
    }

    #[test]
    fn test_scroll_y() {
        let mut a = Address(0);
        a.scroll_y();
        assert_eq!(a.coarse_y(), 0);
        assert_eq!(a.fine_y(), 1);

        a.set_fine_y(7);
        a.scroll_y();
        assert_eq!(a.coarse_y(), 1);
        assert_eq!(a.fine_y(), 0);

        a.set_fine_y(7);
        a.set_coarse_y(29);
        a.set_nametable(0);
        a.scroll_y();
        assert_eq!(a.coarse_y(), 0);
        assert_eq!(a.fine_y(), 0);
        assert_eq!(a.nametable(), 2);

        a.set_fine_y(7);
        a.set_coarse_y(30);
        a.scroll_y();
        assert_eq!(a.coarse_y(), 31);
        assert_eq!(a.fine_y(), 0);

        a.set_fine_y(7);
        a.set_coarse_y(31);
        a.set_nametable(0);
        a.scroll_y();
        assert_eq!(a.coarse_y(), 0);
        assert_eq!(a.fine_y(), 0);
        assert_eq!(a.nametable(), 0);
    }

}
