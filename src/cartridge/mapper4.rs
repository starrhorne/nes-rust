// Mapper4 implements ines mapper 4 (MMC3)
// https://wiki.nesdev.com/w/index.php/MMC3

use super::CartridgeData;
use super::Mapper;
use super::Mirroring;
use super::pager::Page;
use super::pager::PageSize;

pub struct Mapper4 {
    data: CartridgeData,
    registers: [usize; 8],
    index: usize,
    prg_mode: bool,
    chr_mode: bool,
    mirroring: Mirroring,
    irq_counter: u8,
    irq_period: u8,
    irq_enabled: bool,
    irq_reset: bool,
    irq_flag: bool,
}

impl Mapper4 {
    pub fn new(data: CartridgeData) -> Self {
        Mapper4 {
            data: data,
            registers: [0; 8],
            index: 0,
            prg_mode: false,
            chr_mode: false,
            mirroring: Mirroring::Horizontal,
            irq_counter: 0,
            irq_period: 0,
            irq_enabled: false,
            irq_reset: false,
            irq_flag: false,
        }
    }
}

impl Mapper for Mapper4 {
    fn read_prg_byte(&self, address: u16) -> u8 {
        match (address, self.prg_mode) {
            (0x6000...0x7FFF, _) => self.data
                .prg_ram
                .read(Page::First(PageSize::EightKb), address - 0x6000),
            (0x8000...0x9FFF, false) => self.data.prg_rom.read(
                Page::Number(self.registers[6], PageSize::EightKb),
                address - 0x8000,
            ),
            (0x8000...0x9FFF, true) => self.data
                .prg_rom
                .read(Page::FromEnd(1, PageSize::EightKb), address - 0x8000),
            (0xA000...0xBFFF, _) => self.data.prg_rom.read(
                Page::Number(self.registers[7], PageSize::EightKb),
                address - 0xA000,
            ),
            (0xC000...0xDFFF, false) => self.data
                .prg_rom
                .read(Page::FromEnd(1, PageSize::EightKb), address - 0xC000),
            (0xC000...0xDFFF, true) => self.data.prg_rom.read(
                Page::Number(self.registers[6], PageSize::EightKb),
                address - 0xC000,
            ),
            (0xE000...0xFFFF, _) => self.data
                .prg_rom
                .read(Page::FromEnd(0, PageSize::EightKb), address - 0xE000),
            (a, _m) => panic!("bad address: {:04X}", a),
        }
    }

    fn write_prg_byte(&mut self, address: u16, value: u8) {
        match (address, address % 2) {
            (0x6000...0x7FFF, _) => {
                self.data
                    .prg_ram
                    .write(Page::First(PageSize::EightKb), address - 0x6000, value)
            }
            (0x8000...0x9FFF, 0) => {
                self.index = value as usize & 0b111;
                self.prg_mode = value & 0b0100_0000 != 0;
                self.chr_mode = value & 0b1000_0000 != 0;
            }
            (0x8000...0x9FFF, 1) => {
                self.registers[self.index] = value as usize;
            }
            (0xA000...0xBFFF, 0) => {
                self.mirroring = if value & 1 == 0 {
                    Mirroring::Vertical
                } else {
                    Mirroring::Horizontal
                };
            }
            (0xC000...0xDFFF, 0) => self.irq_period = value,
            (0xC000...0xDFFF, 1) => self.irq_reset = true,
            (0xE000...0xFFFF, 0) => {
                self.irq_enabled = false;
                self.irq_flag = false;
            }
            (0xF000...0xFFFF, 1) => self.irq_enabled = true,

            _ => (),
        }
    }
    // $0000-$03FF 	R0 AND $FE 	R2
    // $0400-$07FF 	R0 OR 1 	R3
    // $0800-$0BFF 	R1 AND $FE 	R4
    // $0C00-$0FFF 	R1 OR 1 	R5
    // $1000-$13FF 	R2 	R0 AND $FE
    // $1400-$17FF 	R3 	R0 OR 1
    // $1800-$1BFF 	R4 	R1 AND $FE
    // $1C00-$1FFF 	R5 	R1 OR 1
    fn read_chr_byte(&self, address: u16) -> u8 {
        let bank = match (address, self.chr_mode) {
            (0x0000...0x03FF, false) => self.registers[0] & !1,
            (0x0000...0x03FF, true) => self.registers[2],
            (0x0400...0x07FF, false) => self.registers[0] | 1,
            (0x0400...0x07FF, true) => self.registers[3],
            (0x0800...0x0BFF, false) => self.registers[1] & !1,
            (0x0800...0x0BFF, true) => self.registers[4],
            (0x0C00...0x0FFF, false) => self.registers[1] | 1,
            (0x0C00...0x0FFF, true) => self.registers[5],

            (0x1000...0x13FF, false) => self.registers[2],
            (0x1000...0x13FF, true) => self.registers[0] & !1,
            (0x1400...0x17FF, false) => self.registers[3],
            (0x1400...0x17FF, true) => self.registers[0] | 1,
            (0x1800...0x1BFF, false) => self.registers[4],
            (0x1800...0x1BFF, true) => self.registers[1] & !1,
            (0x1C00...0x1FFF, false) => self.registers[5],
            (0x1C00...0x1FFF, true) => self.registers[1] | 1,
            _ => panic!(),
        };

        self.data
            .chr_rom
            .read(Page::Number(bank, PageSize::OneKb), address % 0x0400)
    }

    fn write_chr_byte(&mut self, _: u16, _: u8) {}

    fn mirroring(&self) -> Mirroring {
        self.mirroring
    }

    fn irq_flag(&self) -> bool {
        self.irq_flag
    }
    fn signal_scanline(&mut self) {
        if self.irq_counter == 0 || self.irq_reset {
            if self.irq_enabled {
                self.irq_flag = true;
            }
            self.irq_counter = self.irq_period;
        } else {
            self.irq_counter -= 1;
        }
    }
}
