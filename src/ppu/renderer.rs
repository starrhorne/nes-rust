use super::PpuResult;
use super::Registers;
use super::colors::RGB;
use super::nth_bit;
use super::sprite::Sprite;

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct BitPlane<T> {
    pub low: T,
    pub high: T,
}

pub struct Renderer {
    pub background_latch: BitPlane<u8>,
    pub background_shift: BitPlane<u16>,
    pub attribute_latch: BitPlane<u8>,
    pub attribute_shift: BitPlane<u8>,
    pub scanline: usize,
    pub dot: usize,
    pub odd_frame: bool,
    pub scratch_address: u16,
    pub nametable_entry: u8,
    pub attribute_entry: u8,
    pub primary_oam: Vec<Sprite>,
    pub secondary_oam: Vec<Sprite>,
    pub pixels: Vec<u32>,
}

impl Renderer {
    pub fn new() -> Self {
        let mut r = Renderer {
            background_latch: BitPlane { low: 0, high: 0 },
            background_shift: BitPlane { low: 0, high: 0 },
            attribute_latch: BitPlane { low: 0, high: 0 },
            attribute_shift: BitPlane { low: 0, high: 0 },
            scanline: 0,
            dot: 0,
            odd_frame: false,
            primary_oam: Vec::with_capacity(8),
            secondary_oam: Vec::with_capacity(8),
            scratch_address: 0,
            nametable_entry: 0,
            attribute_entry: 0,
            pixels: Vec::with_capacity(256 * 240),
        };
        r.reset();
        r
    }

    pub fn clear_pixels(&mut self) {
        self.pixels = vec![0; self.pixels.capacity()]
    }

    pub fn reset(&mut self) {
        self.odd_frame = false;
        self.scanline = 0;
        self.dot = 0;
        self.primary_oam.clear();
        self.secondary_oam.clear();
        self.clear_pixels();
    }

    pub fn tick(&mut self, registers: &mut Registers) -> PpuResult {
        let mut r = match (self.scanline, self.dot) {
            (0...239, _) => {
                self.tick_sprites(false, registers);
                self.tick_pixel(registers);
                self.tick_background(false, registers);
                self.tick_result(registers)
            }
            (261, _) => {
                self.tick_sprites(true, registers);
                self.tick_pixel(registers);
                self.tick_background(true, registers);
                self.tick_result(registers)
            }
            (240, 0) => PpuResult::Draw,
            (241, 1) => {
                if !registers.vblank_suppress {
                    registers.status.set_vblank(true);
                    if registers.control.nmi_on_vblank() {
                        PpuResult::Nmi
                    } else {
                        PpuResult::None
                    }
                } else {
                    PpuResult::None
                }
            }
            (_, _) => PpuResult::None,
        };

        if registers.status.vblank() && registers.force_nmi && !registers.vblank_suppress {
            if let PpuResult::None = r {
                r = PpuResult::Nmi;
            } else {
                panic!();
            }
        }
        registers.force_nmi = false;
        registers.vblank_suppress = false;

        r
    }

    fn tick_sprites(&mut self, pre: bool, registers: &mut Registers) {
        match self.dot {
            1 => {
                self.secondary_oam.clear();
                if pre {
                    registers.status.set_sprite_overflow(false);
                    registers.status.set_sprite_zero_hit(false);
                }
            }
            257 => self.eval_sprites(registers), // TOD - should set oamaddr to 0?
            321 => self.load_sprites(registers),
            _ => (),
        }
    }

    fn tick_pixel(&mut self, registers: &mut Registers) {
        match self.dot {
            2...257 | 322...337 => {
                let x = self.dot - 2;
                let y = self.scanline;
                if let Some(color) = self.render_pixel(x, y, registers) {
                    self.set_pixel(x, y, color, registers);
                }
                self.shift();
            }
            _ => (),
        }
    }

    fn tick_background(&mut self, pre: bool, registers: &mut Registers) {
        match self.dot {
            2...255 | 322...337 => match self.dot % 8 {
                1 => {
                    self.scratch_address = registers.v_address.nametable_address();
                    self.reload_shift_registers();
                }
                2 => {
                    self.nametable_entry = registers.vram.read_byte(self.scratch_address);
                }
                3 => {
                    self.scratch_address = registers.v_address.attribute_address();
                }
                4 => {
                    self.attribute_entry = registers.vram.read_byte(self.scratch_address);
                    if registers.v_address.coarse_y() & 2 != 0 {
                        self.attribute_entry >>= 4;
                    }
                    if registers.v_address.coarse_x() & 2 != 0 {
                        self.attribute_entry >>= 2;
                    }
                }
                5 => {
                    self.scratch_address = registers.control.background_tile_base()
                        + registers.v_address.tile_offset(self.nametable_entry);
                }
                6 => {
                    self.background_latch.low = registers.vram.read_byte(self.scratch_address);
                }
                7 => {
                    self.scratch_address += 8;
                }
                0 => {
                    self.background_latch.high = registers.vram.read_byte(self.scratch_address);

                    if registers.mask.rendering() {
                        registers.v_address.scroll_x();
                    }
                }
                _ => panic!("Impossible math"),
            },
            256 => {
                self.background_latch.high = registers.vram.read_byte(self.scratch_address);
                if registers.mask.rendering() {
                    registers.v_address.scroll_y();
                }
            }
            257 => {
                self.reload_shift_registers();
                if registers.mask.rendering() {
                    registers.v_address.copy_x(registers.t_address);
                }
            }
            280...304 => if pre {
                if registers.mask.rendering() {
                    registers.v_address.copy_y(registers.t_address);
                }
            },
            1 => {
                self.scratch_address = registers.v_address.nametable_address();
                if pre {
                    registers.status.set_vblank(false);
                }
            }
            321 | 339 => {
                self.scratch_address = registers.v_address.nametable_address();
            }
            338 => {
                self.nametable_entry = registers.vram.read_byte(self.scratch_address);
            }
            340 => {
                self.nametable_entry = registers.vram.read_byte(self.scratch_address);
                if pre && registers.mask.rendering() && self.odd_frame {
                    self.dot += 1;
                }
            }

            _ => (),
        }
    }

    fn tick_result(&self, registers: &mut Registers) -> PpuResult {
        if self.dot == 260 && registers.mask.rendering() {
            PpuResult::Scanline
        } else {
            PpuResult::None
        }
    }

    fn render_pixel(&mut self, x: usize, y: usize, registers: &mut Registers) -> Option<u8> {
        if y < 240 && x < 256 {
            let background_color = self.render_background_pixel(x, registers);
            let (sprite_color, sprite_behind, possible_zero_hit) =
                self.render_sprite_pixel(x, registers);

            if possible_zero_hit && background_color != 0 {
                registers.status.set_sprite_zero_hit(true);
            }

            let colors = if sprite_behind {
                [background_color, sprite_color]
            } else {
                [sprite_color, background_color]
            };

            Some(if colors[0] == 0 { colors[1] } else { colors[0] })
        } else {
            None
        }
    }

    fn render_background_pixel(&self, x: usize, registers: &mut Registers) -> u8 {
        if !registers.mask.rendering_background(x) {
            return 0;
        };
        let mut r = nth_bit(self.background_shift.high, 15 - registers.fine_x) << 1
            | nth_bit(self.background_shift.low, 15 - registers.fine_x);

        if r != 0 {
            r |= (nth_bit(self.attribute_shift.high, 7 - registers.fine_x) << 1
                | nth_bit(self.attribute_shift.low, 7 - registers.fine_x)) << 2;
        }
        r
    }

    fn render_sprite_pixel(&mut self, x: usize, registers: &mut Registers) -> (u8, bool, bool) {
        if !registers.mask.rendering_sprites(x) {
            return (0, false, false);
        };

        let mut color = 0;
        let mut behind = false;
        let mut possible_zero_hit = false;

        for s in self.primary_oam.iter().rev() {
            let sci = s.color_index(x);

            if sci != 0 {
                if s.oam_index == 0 && x != 255 {
                    possible_zero_hit = true;
                }
                color = 0b1_00_00 | s.status.palette() << 2 | sci;
                behind = s.status.behind_background();
            }
        }

        (color, behind, possible_zero_hit)
    }

    pub fn step(&mut self) {
        self.dot += 1;
        if self.dot >= 341 {
            self.dot %= 341;
            self.scanline += 1;
            if self.scanline > 261 {
                self.scanline = 0;
                self.odd_frame = !self.odd_frame;
            }
        }
    }

    // TODO - Need to use OAMADDR to determine what is sprite 0: The value of OAMADDR when sprite evaluation starts at
    // tick 65 of the visible scanlines will determine where in OAM sprite evaluation starts, and hence which sprite gets
    // treated as sprite 0. The first OAM entry to be checked during sprite evaluation is the one starting at OAM[OAMADDR].
    fn eval_sprites(&mut self, registers: &mut Registers) {
        self.secondary_oam.clear();
        for i in 0..64 {
            let address = i * 4;
            let sprite = Sprite::new(i, &registers.oam_ram[address..address + 4]);

            // There's a subtle NES detail at play here. We're loading sprites for the NEXT scanline,
            // but we're comparing `sprite.y` to the CURRENT scanline. This is because `sprite.y` values
            // are always offset by 1. So to draw a sprite on scanline 1, you set its Y to 0.
            if self.scanline >= sprite.y as usize
                && self.scanline < sprite.y as usize + registers.control.sprite_height() as usize
            {
                if self.secondary_oam.len() == 8 {
                    registers.status.set_sprite_overflow(true);
                    break;
                }
                self.secondary_oam.push(sprite);
            }
        }
    }

    fn load_sprites(&mut self, registers: &mut Registers) {
        let mut sprites = self.secondary_oam.clone();
        for sprite in sprites.iter_mut() {
            let tile_address = sprite.tile_address(self.scanline, registers.control);
            sprite.data_low = registers.vram.read_byte(tile_address);
            sprite.data_high = registers.vram.read_byte(tile_address + 8);
        }
        self.primary_oam = sprites;
    }

    fn reload_shift_registers(&mut self) {
        self.background_shift.low =
            (self.background_shift.low & 0xFF00) | self.background_latch.low as u16;
        self.background_shift.high =
            (self.background_shift.high & 0xFF00) | self.background_latch.high as u16;
        self.attribute_latch.low = self.attribute_entry & 1;
        self.attribute_latch.high = (self.attribute_entry & 2) >> 1;
    }

    fn shift(&mut self) {
        self.background_shift.low <<= 1;
        self.background_shift.high <<= 1;
        self.attribute_shift.low = self.attribute_shift.low << 1 | self.attribute_latch.low;
        self.attribute_shift.high = self.attribute_shift.high << 1 | self.attribute_latch.high;
    }

    fn set_pixel(&mut self, x: usize, y: usize, color_index: u8, registers: &mut Registers) {
        let pixel_index = y * 256 + x;
        let palette_offset = if registers.mask.rendering() {
            color_index as u16
        } else {
            0
        };
        let rgb_index = registers.vram.read_byte(0x3f00 + palette_offset) as usize;
        self.pixels[pixel_index] = RGB[rgb_index]
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::cell::RefCell;
    use std::rc::Rc;
    use ppu::mask::Mask;
    use cartridge::Cartridge;

    #[test]
    fn test_evaluate_sprites() {
        let mut regs = Registers::new();
        let mut renderer = Renderer::new();
        renderer.scanline = 10;
        regs.oam_ram[0] = 10;
        regs.oam_ram[4] = 10 - 7;
        regs.oam_ram[8] = 10 - 8;
        regs.oam_ram[12] = 11;
        renderer.eval_sprites(&mut regs);
        assert_eq!(renderer.secondary_oam.len(), 2);
    }

    #[test]
    fn test_sprite_overflow() {
        let mut regs = Registers::new();
        let mut renderer = Renderer::new();
        renderer.scanline = 10;
        for i in 0..8 {
            regs.oam_ram[i * 4] = 10;
        }
        renderer.eval_sprites(&mut regs);
        assert_eq!(renderer.secondary_oam.len(), 8);
        assert_eq!(regs.status.sprite_overflow(), false);

        regs.oam_ram[8 * 4] = 10;
        renderer.eval_sprites(&mut regs);
        assert_eq!(renderer.secondary_oam.len(), 8);
        assert_eq!(regs.status.sprite_overflow(), true);
    }

    fn build_cartridge() -> Rc<RefCell<Cartridge>> {
        let mut data = vec![
            0x4e,
            0x45,
            0x53,
            0x1a,
            0x02, // Two pages of PRG-ROM
            0x00, // Zero pages CHR-ROM means use CHR-RAM
            0x00,
            0x00,
            0x01, // One page of PRG-RAM
            0x00,
            0x00,
            0x00,
            0x00,
            0x00,
            0x00,
            0x00,
        ];

        // add the PRG-ROM
        data.extend_from_slice(&[0u8; 2 * 0x4000]);

        Rc::new(RefCell::new(Cartridge::new(&data)))
    }

    #[test]
    fn test_load_sprites() {
        let mut regs = Registers::new();
        let mut renderer = Renderer::new();

        regs.vram.set_cartridge(build_cartridge());

        for i in 0..256 {
            regs.vram.write_byte(i, i as u8);
        }

        renderer.secondary_oam.push(Sprite::new(0, &[5, 3, 1, 2]));
        renderer.scanline = 6;
        renderer.load_sprites(&mut regs);

        assert_eq!(renderer.secondary_oam[0].x, renderer.primary_oam[0].x);
        assert_eq!(renderer.secondary_oam[0].y, renderer.primary_oam[0].y);
        assert_eq!(
            renderer.secondary_oam[0].status,
            renderer.primary_oam[0].status
        );
        assert_eq!(
            renderer.secondary_oam[0].tile_index,
            renderer.primary_oam[0].tile_index
        );

        assert_eq!(
            renderer.primary_oam[0].data_low,
            renderer.primary_oam[0].tile_address(renderer.scanline, regs.control) as u8
        );

        assert_eq!(
            renderer.primary_oam[0].data_high,
            renderer.primary_oam[0].tile_address(renderer.scanline, regs.control) as u8 + 8
        );
    }

    #[test]
    fn test_step() {
        let mut renderer = Renderer::new();
        renderer.dot = 0;
        renderer.scanline = 0;
        renderer.step();
        assert_eq!(renderer.dot, 1);
        assert_eq!(renderer.scanline, 0);

        renderer.dot = 340;
        renderer.scanline = 0;
        renderer.step();
        assert_eq!(renderer.dot, 0);
        assert_eq!(renderer.scanline, 1);

        renderer.dot = 340;
        renderer.scanline = 261;
        renderer.step();
        assert_eq!(renderer.dot, 0);
        assert_eq!(renderer.scanline, 0);
        assert_eq!(renderer.odd_frame, true);
    }

    #[test]
    fn test_render_background_pixel() {
        let mut regs = Registers::new();
        let mut renderer = Renderer::new();
        regs.mask = Mask(0b0001_1110); // Show all sprites and bg
        renderer.background_shift.high = 0b1010_0000_0000_0000;
        renderer.background_shift.low = 0b1100_0000_0000_0000;
        renderer.attribute_shift.high = 0b1010_0000;
        renderer.attribute_shift.low = 0b1100_0000;
        regs.fine_x = 0;
        assert_eq!(renderer.render_background_pixel(0, &mut regs), 0b1111);
        regs.mask = Mask(0b0001_1100); // Hide leftmost 8 px of bg
        assert_eq!(renderer.render_background_pixel(0, &mut regs), 0);
        assert_eq!(renderer.render_background_pixel(8, &mut regs), 0b1111);
        regs.mask = Mask(0b0001_0110); // Hide all bg
        assert_eq!(renderer.render_background_pixel(0, &mut regs), 0);
    }

    #[test]
    fn test_render_sprite_pixel() {
        let mut regs = Registers::new();
        let mut renderer = Renderer::new();

        let mut s = Sprite::new(0, &[0, 0, 0, 0]);
        s.data_low = 0b0100_0000;
        s.data_high = 0b0100_0000;
        renderer.primary_oam.push(s);

        // Bit 5 below sets the "behind bg" flag on the sprite
        let mut s = Sprite::new(1, &[0, 0, 0b0010_0011, 0]);
        s.data_low = 0b0000_0000;
        s.data_high = 0b0001_0000;
        renderer.primary_oam.push(s);

        let mut s = Sprite::new(2, &[0, 0, 0, 0]);
        s.data_low = 0b0100_0000;
        s.data_high = 0b0000_0000;
        renderer.primary_oam.push(s);

        regs.mask = Mask(0b0000_1110); // Hide Sprites
        assert_eq!(
            renderer.render_sprite_pixel(0, &mut regs),
            (0, false, false)
        );

        regs.mask = Mask(0b0001_1010); // Hide Left 8 Px of Sprites
        assert_eq!(
            renderer.render_sprite_pixel(0, &mut regs),
            (0, false, false)
        );

        regs.mask = Mask(0b0001_1110); // Show all sprites and bg
        assert_eq!(
            renderer.render_sprite_pixel(1, &mut regs),
            (0b1_00_11, false, true)
        );
        assert_eq!(
            renderer.render_sprite_pixel(3, &mut regs),
            (0b1_11_10, true, false)
        );
    }

    #[test]
    fn test_reload_shift() {
        let mut renderer = Renderer::new();
        renderer.background_shift.low = 0b1010_1010_1010_1010;
        renderer.background_shift.high = 0b0101_0101_0101_0101;
        renderer.background_latch.low = 0b0000_0001;
        renderer.background_latch.high = 0b0000_0010;
        renderer.attribute_entry = 0b11;

        renderer.reload_shift_registers();
        assert_eq!(renderer.background_shift.low, 0b1010_1010_0000_0001);
        assert_eq!(renderer.background_shift.high, 0b0101_0101_0000_0010);
        assert_eq!(renderer.attribute_latch.low, 1);
        assert_eq!(renderer.attribute_latch.high, 1);
    }

    #[test]
    fn test_shift() {
        let mut renderer = Renderer::new();
        renderer.background_shift.low = 0b1010_1010_1010_1010;
        renderer.background_shift.high = 0b0101_0101_0101_0101;
        renderer.attribute_latch.low = 0;
        renderer.attribute_latch.high = 1;
        renderer.shift();
        assert_eq!(renderer.background_shift.low, 0b0101_0101_0101_0100);
        assert_eq!(renderer.background_shift.high, 0b1010_1010_1010_1010);
        assert_eq!(renderer.attribute_shift.low, 0);
        assert_eq!(renderer.attribute_shift.high, 1);
    }

    #[test]
    fn test_render_pixel_transparent_sprite_front() {
        let mut regs = Registers::new();
        let mut renderer = Renderer::new();
        regs.mask = Mask(0b0001_1110); // Show all sprites and bg
        renderer.background_shift.high = 0b1111_0000_0000_0000;
        renderer.background_shift.low = 0b1111_0000_0000_0000;
        assert_eq!(renderer.render_background_pixel(0, &mut regs), 0b11);

        let mut s = Sprite::new(0, &[0, 0, 0, 0]);
        s.data_low = 0b0100_0000;
        s.data_high = 0b0100_0000;
        renderer.primary_oam.push(s);
        assert_eq!(
            renderer.render_sprite_pixel(0, &mut regs),
            (0, false, false)
        );
        assert_eq!(renderer.render_pixel(0, 0, &mut regs), Some(0b11));
        assert_eq!(regs.status.sprite_zero_hit(), false);
    }

    #[test]
    fn test_render_pixel_opaque_sprite_front() {
        let mut regs = Registers::new();
        let mut renderer = Renderer::new();
        regs.mask = Mask(0b0001_1110); // Show all sprites and bg
        renderer.background_shift.high = 0b1111_0000_0000_0000;
        renderer.background_shift.low = 0b1111_0000_0000_0000;
        assert_eq!(renderer.render_background_pixel(0, &mut regs), 0b11);

        let mut s = Sprite::new(0, &[0, 0, 0, 0]);
        s.data_low = 0b1000_0000;
        s.data_high = 0b0000_0000;
        renderer.primary_oam.push(s);
        assert_eq!(
            renderer.render_sprite_pixel(0, &mut regs),
            (0b1_00_01, false, true)
        );

        assert_eq!(renderer.render_pixel(0, 0, &mut regs), Some(0b1_00_01));
        assert_eq!(regs.status.sprite_zero_hit(), true);
    }

    #[test]
    fn test_render_pixel_opaque_sprite_behind() {
        let mut regs = Registers::new();
        let mut renderer = Renderer::new();
        regs.mask = Mask(0b0001_1110); // Show all sprites and bg
        renderer.background_shift.high = 0b1111_0000_0000_0000;
        renderer.background_shift.low = 0b1111_0000_0000_0000;
        assert_eq!(renderer.render_background_pixel(0, &mut regs), 0b11);

        let mut s = Sprite::new(0, &[0, 0, 0b0010_0000, 0]);
        s.data_low = 0b1000_0000;
        s.data_high = 0b0000_0000;
        renderer.primary_oam.push(s);
        assert_eq!(
            renderer.render_sprite_pixel(0, &mut regs),
            (0b1_00_01, true, true)
        );
        assert_eq!(renderer.render_pixel(0, 0, &mut regs), Some(0b11));
        assert_eq!(regs.status.sprite_zero_hit(), true);
    }

}
