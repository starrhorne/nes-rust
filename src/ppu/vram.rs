use cartridge::{Cartridge, Mirroring};
use std::cell::RefCell;
use std::rc::Rc;

const NAMETABLE_SIZE: usize = 0x400;
const PALETTE_SIZE: usize = 0x20;

pub struct Vram {
    pub nametables: [u8; 2 * NAMETABLE_SIZE],
    pub palettes: [u8; PALETTE_SIZE],
    read_buffer: u8,
    cartridge: Option<Rc<RefCell<Cartridge>>>,
}

impl Vram {
    pub fn new() -> Self {
        Vram {
            nametables: [0; 2 * NAMETABLE_SIZE],
            palettes: [0; PALETTE_SIZE],
            read_buffer: 0,
            cartridge: None,
        }
    }

    pub fn reset(&mut self) {
        self.nametables = [0xFF; 0x800];
        self.palettes = [0; 0x20];
        self.cartridge = None;
    }

    pub fn set_cartridge(&mut self, cartridge: Rc<RefCell<Cartridge>>) {
        self.cartridge = Some(cartridge);
    }

    pub fn mirroring(&self) -> Mirroring {
        if let Some(ref c) = self.cartridge {
            c.borrow().mirroring()
        } else {
            Mirroring::None
        }
    }

    pub fn write_byte(&mut self, address: u16, value: u8) {
        let mirroring = self.mirroring();
        match address {
            0x0000...0x1FFF => match self.cartridge {
                Some(ref c) => c.borrow_mut().write_chr_byte(address, value),
                None => panic!("tried to write to non-existant cartridge memory"),
            },
            0x2000...0x3EFF => self.nametables[mirror_nametable(mirroring, address)] = value,
            0x3F00...0x3FFF => self.palettes[mirror_palette(address)] = value,
            _ => (),
        };
    }

    pub fn read_byte(&mut self, address: u16) -> u8 {
        let mirroring = self.mirroring();
        match address {
            0x0000...0x1FFF => match self.cartridge {
                Some(ref c) => c.borrow().read_chr_byte(address),
                None => panic!("tried to read non-existant cartridge memory"),
            },
            0x2000...0x3EFF => self.nametables[mirror_nametable(mirroring, address)],
            0x3F00...0x3FFF => self.palettes[mirror_palette(address)],
            _ => 0,
        }
    }

    pub fn buffered_read_byte(&mut self, address: u16) -> u8 {
        if address < 0x3F00 {
            let result = self.read_buffer;
            self.read_buffer = self.read_byte(address);
            result
        } else {
            let mirroring = self.mirroring();
            self.read_buffer = self.nametables[mirror_nametable(mirroring, address)];
            self.read_byte(address)
        }
    }
}

fn mirror_nametable(mirroring: Mirroring, address: u16) -> usize {
    let address = address as usize;
    let result = match mirroring {
        Mirroring::None => address - 0x2000,
        Mirroring::Horizontal => ((address / 2) & NAMETABLE_SIZE) + (address % NAMETABLE_SIZE),
        Mirroring::Vertical => address % (2 * NAMETABLE_SIZE),
    };
    result
}

fn mirror_palette(address: u16) -> usize {
    let address = (address as usize) % PALETTE_SIZE;

    match address {
        0x10 | 0x14 | 0x18 | 0x1C => address - 0x10,
        _ => address,
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_read_byte_nametable() {
        let mut v = Vram::new();
        v.nametables[0x201] = 0x11;
        assert_eq!(v.read_byte(0x2201), 0x11);
        assert_eq!(v.read_byte(0x2200), 0);
    }

    #[test]
    fn test_write_byte_nametable() {
        let mut v = Vram::new();
        v.write_byte(0x2201, 0x11);
        assert_eq!(v.nametables[0x201], 0x11);
        assert_eq!(v.nametables[0x200], 0x00);
    }

    #[test]
    fn test_read_byte_palette() {
        let mut v = Vram::new();
        v.palettes[0x09] = 0x22;
        v.palettes[0] = 0x33;
        assert_eq!(v.read_byte(0x3F09), 0x22);
        assert_eq!(v.read_byte(0x3F00), 0x33);
        assert_eq!(v.read_byte(0x3F11), 0);
    }

    #[test]
    fn test_write_byte_palette() {
        let mut v = Vram::new();
        v.write_byte(0x3F09, 0x11);
        assert_eq!(v.palettes[0x09], 0x11);
    }

    fn build_cartridge() -> Rc<RefCell<Cartridge>> {
        let mut data = vec![
            0x4e,
            0x45,
            0x53,
            0x1a,
            0x02, // Two pages of PRG-ROM
            0x01, // One page of CHR-ROM
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

        // add the CHR-ROM
        for i in 0..0x2000u16 {
            data.push(i as u8);
        }

        Rc::new(RefCell::new(Cartridge::new(&data)))
    }

    #[test]
    fn test_read_byte_cartridge() {
        let mut v = Vram::new();
        v.set_cartridge(build_cartridge());
        assert_eq!(v.read_byte(0), 0);
        assert_eq!(v.read_byte(10), 10);
        assert_eq!(v.read_byte(20), 20);
    }

    #[test]
    fn test_buffered_read_byte() {
        let mut v = Vram::new();
        v.nametables[0x201] = 0x11;
        v.nametables[0x202] = 0x12;
        assert_eq!(v.buffered_read_byte(0x2201), 0);
        assert_eq!(v.buffered_read_byte(0x2202), 0x11);
        assert_eq!(v.buffered_read_byte(0x2203), 0x12);
        assert_eq!(v.buffered_read_byte(0x2204), 0);
    }

    #[test]
    fn test_mirror_nametable_horizontally() {
        // Nametable 1 - starting at 0x2000
        assert_eq!(mirror_nametable(Mirroring::Horizontal, 0x2001), 1);
        assert_eq!(mirror_nametable(Mirroring::Horizontal, 0x2201), 0x201);
        assert_eq!(mirror_nametable(Mirroring::Horizontal, 0x2401), 1);
        assert_eq!(mirror_nametable(Mirroring::Horizontal, 0x2601), 0x201);

        // Nametable 1 - mirrored at 0x3000
        assert_eq!(mirror_nametable(Mirroring::Horizontal, 0x3001), 1);
        assert_eq!(mirror_nametable(Mirroring::Horizontal, 0x3201), 0x201);
        assert_eq!(mirror_nametable(Mirroring::Horizontal, 0x3401), 1);
        assert_eq!(mirror_nametable(Mirroring::Horizontal, 0x3601), 0x201);

        // Nametable 2 - starting at 0x2800
        assert_eq!(mirror_nametable(Mirroring::Horizontal, 0x2801), 0x401);
        assert_eq!(mirror_nametable(Mirroring::Horizontal, 0x2A01), 0x601);
        assert_eq!(mirror_nametable(Mirroring::Horizontal, 0x2C01), 0x401);
        assert_eq!(mirror_nametable(Mirroring::Horizontal, 0x2E01), 0x601);

        // Nametable 2 - mirrored at 0x3800
        assert_eq!(mirror_nametable(Mirroring::Horizontal, 0x3801), 0x401);
        assert_eq!(mirror_nametable(Mirroring::Horizontal, 0x3A01), 0x601);
        assert_eq!(mirror_nametable(Mirroring::Horizontal, 0x3C01), 0x401);
        assert_eq!(mirror_nametable(Mirroring::Horizontal, 0x3E01), 0x601);
    }

    #[test]
    fn test_mirror_nametable_vertically() {
        // Nametable 1 - starting at 0x2000
        assert_eq!(mirror_nametable(Mirroring::Vertical, 0x2001), 1);
        assert_eq!(mirror_nametable(Mirroring::Vertical, 0x2201), 0x201);
        assert_eq!(mirror_nametable(Mirroring::Vertical, 0x2801), 1);
        assert_eq!(mirror_nametable(Mirroring::Vertical, 0x2A01), 0x201);

        // Nametable 1 - mirrored at 0x3000
        assert_eq!(mirror_nametable(Mirroring::Vertical, 0x3001), 1);
        assert_eq!(mirror_nametable(Mirroring::Vertical, 0x3201), 0x201);
        assert_eq!(mirror_nametable(Mirroring::Vertical, 0x3801), 1);
        assert_eq!(mirror_nametable(Mirroring::Vertical, 0x3A01), 0x201);

        // Nametable 2 - starting at 0x2400
        assert_eq!(mirror_nametable(Mirroring::Vertical, 0x2401), 0x401);
        assert_eq!(mirror_nametable(Mirroring::Vertical, 0x2601), 0x601);
        assert_eq!(mirror_nametable(Mirroring::Vertical, 0x2C01), 0x401);
        assert_eq!(mirror_nametable(Mirroring::Vertical, 0x2E01), 0x601);

        // Nametable 2 - mirrored at 0x3800
        assert_eq!(mirror_nametable(Mirroring::Vertical, 0x3401), 0x401);
        assert_eq!(mirror_nametable(Mirroring::Vertical, 0x3601), 0x601);
        assert_eq!(mirror_nametable(Mirroring::Vertical, 0x3C01), 0x401);
        assert_eq!(mirror_nametable(Mirroring::Vertical, 0x3E01), 0x601);
    }

    #[test]
    fn test_mirror_palette() {
        assert_eq!(mirror_palette(0x3F01), 1);
        assert_eq!(mirror_palette(0x3F21), 1);
        assert_eq!(mirror_palette(0x3F41), 1);
        assert_eq!(mirror_palette(0x3F11), 0x11);
        // Test mirroring of 0x10
        assert_eq!(mirror_palette(0x3F10), 0);
        assert_eq!(mirror_palette(0x3F30), 0);
        // Test mirroring of 0x14
        assert_eq!(mirror_palette(0x3F14), 4);
        assert_eq!(mirror_palette(0x3F34), 4);
        // Test mirroring of 0x18
        assert_eq!(mirror_palette(0x3F18), 8);
        assert_eq!(mirror_palette(0x3F38), 8);
        // Test mirroring of 0x1c
        assert_eq!(mirror_palette(0x3F1C), 0x0C);
        assert_eq!(mirror_palette(0x3F3C), 0x0C);
    }
}
