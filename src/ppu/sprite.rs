use super::control::Control;
use super::nth_bit;

bitfield!{
    #[derive(Copy, Clone, PartialEq)]
    pub struct SpriteStatus(u8);
    impl Debug;
    pub palette,            _: 1, 0;
    pub behind_background,  _:    5;
    pub flip_x,             _:    6;
    pub flip_y,             _:    7;
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct SpriteTileIndex(u8);

impl SpriteTileIndex {
    pub fn base(&self) -> u16 {
        (0x1000 * (self.0 & 1) as u16)
    }
    pub fn large_offset(&self) -> u16 {
        (16 * (self.0 & !1) as u16)
    }
    pub fn small_offset(&self) -> u16 {
        (16 * self.0 as u16)
    }
}

#[derive(Debug, Copy, Clone)]
pub struct Sprite {
    pub x: u8,
    pub y: u8,
    pub status: SpriteStatus,
    pub tile_index: SpriteTileIndex,
    pub data_low: u8,
    pub data_high: u8,
    pub oam_index: usize,
}

impl Sprite {
    pub fn new(oam_index: usize, bytes: &[u8]) -> Self {
        Sprite {
            y: bytes[0],
            tile_index: SpriteTileIndex(bytes[1]),
            status: SpriteStatus(bytes[2]),
            x: bytes[3],
            data_low: 0,
            data_high: 0,
            oam_index,
        }
    }

    pub fn tile_address(&self, scanline: usize, control: Control) -> u16 {
        let tile_address = if control.large_sprites() {
            self.tile_index.base() + self.tile_index.large_offset()
        } else {
            control.sprite_tile_base() + self.tile_index.small_offset()
        };
        // TODO: why mod sprite_height?
        let mut y_offset =
            ((scanline - self.y as usize) as u16 % control.sprite_height() as u16) as u16;

        if self.status.flip_y() {
            y_offset = control.sprite_height() as u16 - 1 - y_offset;
        }

        tile_address + y_offset + if y_offset < 8 { 0 } else { 8 }
    }

    pub fn color_index(&self, x: usize) -> u8 {
        let mut sprite_x = x.wrapping_sub(self.x as usize) as u16;
        if sprite_x < 8 {
            if self.status.flip_x() {
                sprite_x = 7 - sprite_x;
            }
            nth_bit(self.data_high, 7 - sprite_x) << 1 | nth_bit(self.data_low, 7 - sprite_x)
        } else {
            0
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_tile_base() {
        assert_eq!(SpriteTileIndex(0b0000_0000).base(), 0);
        assert_eq!(SpriteTileIndex(0b0000_0001).base(), 0x1000);
    }

    #[test]
    fn test_sprite_tile_offset() {
        assert_eq!(SpriteTileIndex(0b0000_0101).large_offset(), 4 * 16);
        assert_eq!(SpriteTileIndex(0b0000_0101).small_offset(), 5 * 16);
    }

    #[test]
    fn test_tile_address_small_no_flip() {
        let sprite = Sprite::new(0, &[5, 7, 0, 0]);
        assert_eq!(sprite.tile_address(5, Control(0)), 0 + (7 * 16) + (5 - 5));
        assert_eq!(sprite.tile_address(8, Control(0)), 0 + (7 * 16) + (8 - 5));
        assert_eq!(sprite.tile_address(12, Control(0)), 0 + (7 * 16) + (12 - 5));
        assert_eq!(sprite.tile_address(13, Control(0)), 0 + (7 * 16) + (5 - 5));
    }

    #[test]
    fn test_tile_address_small_flip_y() {
        let sprite = Sprite::new(0, &[5, 7, 0b1000_0000, 0]);
        assert_eq!(sprite.tile_address(5, Control(0)), 0 + (7 * 16) + (12 - 5));
        assert_eq!(sprite.tile_address(8, Control(0)), 0 + (7 * 16) + (9 - 5));
        assert_eq!(sprite.tile_address(12, Control(0)), 0 + (7 * 16) + (5 - 5));
        assert_eq!(sprite.tile_address(13, Control(0)), 0 + (7 * 16) + (12 - 5));
    }

    #[test]
    fn test_tile_address_large_no_flip() {
        let sprite = Sprite::new(0, &[5, 7, 0, 0]);
        let c = Control(0b0010_0000);
        assert_eq!(sprite.tile_address(5, c), 0x1000 + (6 * 16) + (5 - 5));
        assert_eq!(sprite.tile_address(8, c), 0x1000 + (6 * 16) + (8 - 5));
        assert_eq!(sprite.tile_address(12, c), 0x1000 + (6 * 16) + (12 - 5));
        assert_eq!(sprite.tile_address(13, c), 0x1000 + (6 * 16) + 8 + (13 - 5));
        assert_eq!(sprite.tile_address(16, c), 0x1000 + (6 * 16) + 8 + (16 - 5));
        assert_eq!(sprite.tile_address(19, c), 0x1000 + (6 * 16) + 8 + (19 - 5));
        assert_eq!(sprite.tile_address(21, c), 0x1000 + (6 * 16) + (5 - 5));
    }

    #[test]
    fn test_tile_address_large_flip_y() {
        let sprite = Sprite::new(0, &[5, 7, 0b1000_0000, 0]);
        let c = Control(0b0010_0000);
        assert_eq!(sprite.tile_address(5, c), 0x1000 + (6 * 16) + 8 + (20 - 5));
        assert_eq!(sprite.tile_address(6, c), 0x1000 + (6 * 16) + 8 + (19 - 5));
        assert_eq!(sprite.tile_address(20, c), 0x1000 + (6 * 16) + 0 + (5 - 5));
    }

    #[test]
    fn test_color_index() {
        let mut sprite = Sprite::new(0, &[0, 0, 0, 4]);
        sprite.data_low = 0b1000_0010;
        sprite.data_high = 0b0100_0010;
        assert_eq!(sprite.color_index(4 + 6), 3);
        assert_eq!(sprite.color_index(4 + 0), 1);
        assert_eq!(sprite.color_index(4 + 1), 2);
    }

    #[test]
    fn test_color_index_flip_x() {
        let mut sprite = Sprite::new(0, &[0, 0, 0b0100_0000, 4]);
        sprite.data_low = 0b1000_0010;
        sprite.data_high = 0b0100_0010;
        assert_eq!(sprite.color_index(4 + 7 - 6), 3);
        assert_eq!(sprite.color_index(4 + 7 - 0), 1);
        assert_eq!(sprite.color_index(4 + 7 - 1), 2);
    }
}
