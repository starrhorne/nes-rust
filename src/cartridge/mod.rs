mod pager;
mod cartridge_header;
mod cartridge_data;
mod mapper;
mod mapper0;
mod mapper1;
mod mapper2;
mod mapper3;
mod mapper4;

use self::cartridge_data::CartridgeData;
use self::mapper::Mapper;
use self::mapper0::Mapper0;
use self::mapper1::Mapper1;
use self::mapper2::Mapper2;
use self::mapper3::Mapper3;
use self::mapper4::Mapper4;

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Mirroring {
    Vertical,
    Horizontal,
    None,
}

pub struct Cartridge {
    mapper: Box<Mapper>,
}

impl Cartridge {
    pub fn new(data: &[u8]) -> Self {
        let data = CartridgeData::new(data);

        let mapper: Box<Mapper> = match data.header.mapper_number {
            0 => Box::new(Mapper0::new(data)),
            1 => Box::new(Mapper1::new(data)),
            2 => Box::new(Mapper2::new(data)),
            3 => Box::new(Mapper3::new(data)),
            4 => Box::new(Mapper4::new(data)),
            n => panic!("Mapper {} not implemented", n),
        };

        Cartridge { mapper: mapper }
    }

    pub fn signal_scanline(&mut self) {
        self.mapper.signal_scanline();
    }

    pub fn read_prg_byte(&self, address: u16) -> u8 {
        self.mapper.read_prg_byte(address)
    }

    pub fn write_prg_byte(&mut self, address: u16, value: u8) {
        self.mapper.write_prg_byte(address, value);
    }

    pub fn read_chr_byte(&self, address: u16) -> u8 {
        self.mapper.read_chr_byte(address)
    }

    pub fn write_chr_byte(&mut self, address: u16, value: u8) {
        self.mapper.write_chr_byte(address, value)
    }

    pub fn mirroring(&self) -> Mirroring {
        self.mapper.mirroring()
    }

    pub fn irq_flag(&self) -> bool {
        self.mapper.irq_flag()
    }
}

#[cfg(test)]
mod ppu_test {
    use super::*;
    fn build_cartridge(chr_ram: bool) -> Cartridge {
        let mut data = vec![
            0x4e,
            0x45,
            0x53,
            0x1a,
            0x02,                        // Two pages of PRG-ROM
            if chr_ram { 0 } else { 1 }, // One page of CHR-ROM
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
        for i in 0..0x8000u16 / 2 {
            data.push((i >> 8) as u8);
            data.push(i as u8);
        }

        if !chr_ram {
            // add the CHR-ROM
            for i in 0..0x2000u16 / 2 {
                data.push((i >> 8) as u8);
                data.push(i as u8);
            }
        }

        Cartridge::new(&data)
    }

    #[test]
    fn test_read_prg_rom() {
        let cartridge = build_cartridge(false);
        for i in 0..0x8000u16 {
            if i % 2 == 0 {
                assert_eq!(cartridge.read_prg_byte(0x8000 + i), ((i / 2) >> 8) as u8);
            } else {
                assert_eq!(cartridge.read_prg_byte(0x8000 + i), (i / 2) as u8);
            }
        }
    }

    #[test]
    fn test_prg_ram() {
        let mut cartridge = build_cartridge(false);
        for i in 0x6000u16..0x7000u16 {
            cartridge.write_prg_byte(i, i as u8);
            assert_eq!(cartridge.read_prg_byte(i), i as u8);
        }
    }

    #[test]
    fn test_read_chr_rom() {
        let cartridge = build_cartridge(false);
        for i in 0..0x2000u16 {
            if i % 2 == 0 {
                assert_eq!(cartridge.read_chr_byte(i), ((i / 2) >> 8) as u8);
            } else {
                assert_eq!(cartridge.read_chr_byte(i), (i / 2) as u8);
            }
        }
    }

    #[test]
    fn test_chr_ram() {
        let mut cartridge = build_cartridge(true);
        for i in 0..0x2000u16 {
            cartridge.write_chr_byte(i, i as u8);
            assert_eq!(cartridge.read_chr_byte(i), i as u8);
        }
    }
}
