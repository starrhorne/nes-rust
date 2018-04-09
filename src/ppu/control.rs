bitfield!{
    #[derive(Copy, Clone)]
    pub struct Control(u8);
    impl Debug;
    pub nametable,          _: 1, 0;
    pub vertical_increment, _:    2;
    pub sprite_table,       _:    3;
    pub background_table,   _:    4;
    pub large_sprites,      _:    5;
    pub slave,              _:    6;
    pub nmi_on_vblank,      _:    7;
}

impl Control {
    pub fn sprite_height(&self) -> u8 {
        if self.large_sprites() {
            16
        } else {
            8
        }
    }
    pub fn sprite_tile_base(&self) -> u16 {
        self.sprite_table() as u16 * 0x1000
    }

    pub fn background_tile_base(&self) -> u16 {
        self.background_table() as u16 * 0x1000
    }

    pub fn increment_amount(&self) -> u16 {
        if self.vertical_increment() {
            32
        } else {
            1
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_sprite_tile_base() {
        assert_eq!(Control(0b0000_1000).sprite_tile_base(), 0x1000);
        assert_eq!(Control(0b0000_0000).sprite_tile_base(), 0);
    }

    #[test]
    fn test_background_tile_base() {
        assert_eq!(Control(0b0001_0000).background_tile_base(), 0x1000);
        assert_eq!(Control(0b0000_0000).background_tile_base(), 0);
    }

    #[test]
    fn test_increment_amount() {
        assert_eq!(Control(0b0000_0100).increment_amount(), 32);
        assert_eq!(Control(0b0000_0000).increment_amount(), 1);
    }

    #[test]
    fn test_sprite_height() {
        assert_eq!(Control(0b0010_0000).sprite_height(), 16);
        assert_eq!(Control(0b0000_0000).sprite_height(), 8);
    }
}
