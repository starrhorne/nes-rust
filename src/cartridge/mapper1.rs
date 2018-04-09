// Mapper1 implements ines mapper 1 (MMC1)
// https://wiki.nesdev.com/w/index.php/MMC1

use super::CartridgeData;
use super::Mapper;
use super::Mirroring;
use super::pager::Page;
use super::pager::PageSize;

#[derive(Debug, Copy, Clone, PartialEq)]
enum AddressRange {
    Low,
    High,
}

#[derive(Debug, Copy, Clone, PartialEq)]
enum PrgMode {
    Consecutive,
    FixFirst,
    FixLast,
}

#[derive(Debug, Copy, Clone, PartialEq)]
enum ChrMode {
    Consecutive,
    NonConsecutive,
}

bitfield!{
    #[derive(Copy, Clone)]
    struct ControlRegister(u8);
    impl Debug;
    nt_mode_id, _: 1, 0;
    prg_mode_id, _: 3, 2;
    chr_mode_id, _: 4, 4;
}

impl ControlRegister {
    fn mirroring(&self) -> Mirroring {
        match self.nt_mode_id() {
            2 => Mirroring::Vertical,
            3 => Mirroring::Horizontal,
            _ => panic!("Unsupported Mirroring"), // TODO: Implement additional modes
        }
    }

    fn prg_mode(&self) -> PrgMode {
        match self.prg_mode_id() {
            0 | 1 => PrgMode::Consecutive,
            2 => PrgMode::FixFirst,
            3 => PrgMode::FixLast,
            _ => panic!("Impossible"),
        }
    }

    fn chr_mode(&self) -> ChrMode {
        match self.chr_mode_id() {
            1 => ChrMode::NonConsecutive,
            0 => ChrMode::Consecutive,
            _ => panic!("Impossible"),
        }
    }
}

struct ShiftRegister {
    value: u8,
    bit_index: u8,
}

impl ShiftRegister {
    fn new() -> Self {
        ShiftRegister {
            value: 0,
            bit_index: 0,
        }
    }
    fn reset(&mut self) {
        self.value = 0;
        self.bit_index = 0;
    }

    fn push(&mut self, n: u8) -> Option<u8> {
        if n >> 7 == 1 {
            self.reset();
        } else {
            self.value |= (n & 1) << self.bit_index; // Fill bits in order from 0 to 4
            if self.bit_index == 4 {
                let result = self.value;
                self.reset();
                return Some(result);
            }
            self.bit_index += 1;
        }
        None
    }
}

pub struct Mapper1 {
    data: CartridgeData,
    shift: ShiftRegister,
    control: ControlRegister,
    prg_0: usize,
    chr_0: usize,
    chr_1: usize,
}

impl Mapper1 {
    pub fn new(data: CartridgeData) -> Self {
        Mapper1 {
            data,
            shift: ShiftRegister::new(),
            control: ControlRegister(0b0_11_10), // Consecutive_FixLast_Horizontal
            chr_0: 0,
            chr_1: 0,
            prg_0: 0,
        }
    }

    fn write_shift(&mut self, address: u16, value: u8) {
        if let Some(shift_value) = self.shift.push(value) {
            match address {
                0x8000...0x9FFF => self.control = ControlRegister(shift_value),
                0xA000...0xBFFF => self.chr_0 = shift_value as usize & 0b1_1111,
                0xC000...0xDFFF => self.chr_1 = shift_value as usize & 0b1_1111,
                0xE000...0xFFFF => self.prg_0 = shift_value as usize & 0b1111,
                _ => panic!("Invalid address"),
            }
        }
    }

    fn read_paged_prg_ram(&self, offset: u16) -> u8 {
        self.data
            .prg_ram
            .read(Page::First(PageSize::EightKb), offset)
    }

    fn write_paged_prg_ram(&mut self, offset: u16, value: u8) {
        self.data
            .prg_ram
            .write(Page::First(PageSize::EightKb), offset, value);
    }

    fn write_paged_chr_ram(&mut self, address_range: AddressRange, offset: u16, value: u8) {
        let page = match self.control.chr_mode() {
            ChrMode::Consecutive => match address_range {
                AddressRange::Low => Page::Number(self.chr_0, PageSize::FourKb),
                AddressRange::High => Page::Number(self.chr_0 + 1, PageSize::FourKb),
            },
            ChrMode::NonConsecutive => match address_range {
                AddressRange::Low => Page::Number(self.chr_0, PageSize::FourKb), // TODO !? Low bit??
                AddressRange::High => Page::Number(self.chr_1, PageSize::FourKb),
            },
        };
        self.data.chr_ram.write(page, offset, value)
    }

    fn read_paged_prg_rom(&self, address_range: AddressRange, offset: u16) -> u8 {
        let page = match self.control.prg_mode() {
            PrgMode::FixFirst => match address_range {
                AddressRange::Low => Page::First(PageSize::SixteenKb),
                AddressRange::High => Page::Number(self.prg_0, PageSize::SixteenKb),
            },
            PrgMode::FixLast => match address_range {
                AddressRange::Low => Page::Number(self.prg_0, PageSize::SixteenKb),
                AddressRange::High => Page::Last(PageSize::SixteenKb),
            },
            PrgMode::Consecutive => match address_range {
                AddressRange::Low => Page::Number(self.prg_0 & !1, PageSize::SixteenKb),
                AddressRange::High => Page::Number(self.prg_0 | 1, PageSize::SixteenKb),
            },
        };
        self.data.prg_rom.read(page, offset)
    }

    fn read_paged_chr_rom(&self, address_range: AddressRange, offset: u16) -> u8 {
        let page = match self.control.chr_mode() {
            ChrMode::Consecutive => match address_range {
                AddressRange::Low => Page::Number(self.chr_0, PageSize::FourKb),
                AddressRange::High => Page::Number(self.chr_0 + 1, PageSize::FourKb),
            },
            ChrMode::NonConsecutive => match address_range {
                AddressRange::Low => Page::Number(self.chr_0, PageSize::FourKb), // TODO !? Low bit??
                AddressRange::High => Page::Number(self.chr_1, PageSize::FourKb),
            },
        };

        if self.data.header.chr_rom_pages == 0 {
            self.data.chr_ram.read(page, offset)
        } else {
            self.data.chr_rom.read(page, offset)
        }
    }
}

impl Mapper for Mapper1 {
    fn read_prg_byte(&self, address: u16) -> u8 {
        match address {
            0x6000...0x7FFF => self.read_paged_prg_ram(address - 0x6000),
            0x8000...0xBFFF => self.read_paged_prg_rom(AddressRange::Low, address - 0x8000),
            0xC000...0xFFFF => self.read_paged_prg_rom(AddressRange::High, address - 0xC000),
            _ => panic!("bad address"),
        }
    }

    fn write_prg_byte(&mut self, address: u16, value: u8) {
        match address {
            0x6000...0x7FFF => self.write_paged_prg_ram(address - 0x6000, value),
            0x8000...0xFFFF => self.write_shift(address, value),
            _ => panic!("bad address"),
        }
    }

    fn read_chr_byte(&self, address: u16) -> u8 {
        match address {
            0x0000...0x0FFF => self.read_paged_chr_rom(AddressRange::Low, address),
            0x1000...0x1FFF => self.read_paged_chr_rom(AddressRange::High, address - 0x1000),
            _ => panic!("bad address"),
        }
    }

    fn write_chr_byte(&mut self, address: u16, value: u8) {
        match address {
            0x0000...0x0FFF => self.write_paged_chr_ram(AddressRange::Low, address, value),
            0x1000...0x1FFF => {
                self.write_paged_chr_ram(AddressRange::High, address - 0x1000, value)
            }
            _ => panic!("bad address"),
        }
    }

    fn mirroring(&self) -> Mirroring {
        // Todo - what about the mirroring mode from the ines file header?
        self.control.mirroring()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_shift() {
        let mut shift = ShiftRegister::new();
        assert_eq!(None, shift.push(1));
        assert_eq!(None, shift.push(0));
        assert_eq!(None, shift.push(1));
        assert_eq!(None, shift.push(1));
        assert_eq!(Some(0b01101), shift.push(0)); // Fifth bit returns the value and resets
        assert_eq!(None, shift.push(1));
        assert_eq!(None, shift.push(1));
        assert_eq!(None, shift.push(0));
        assert_eq!(None, shift.push(0));
        assert_eq!(Some(0b10011), shift.push(1)); // Try it again to make sure
        assert_eq!(None, shift.push(1));
        assert_eq!(None, shift.push(1));
        assert_eq!(None, shift.push(0b1000_0000)); // high bit causes register to clear
        assert_eq!(None, shift.push(1));
        assert_eq!(None, shift.push(0));
        assert_eq!(None, shift.push(1));
        assert_eq!(None, shift.push(0));
        assert_eq!(Some(0b10101), shift.push(1));
    }

    fn build_cartridge_data() -> CartridgeData {
        let mut data = vec![
            0x4e,
            0x45,
            0x53,
            0x1a,
            0x0F, // 16 x 16kb prg rom
            0x0F, // 16 x 8kb chr rom
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
        for i in 0..0x8000 * 16 {
            data.push(i as u8);
        }

        // add the CHR-ROM
        for i in 0..0x4000 * 16 {
            data.push(i as u8);
        }

        CartridgeData::new(&data)
    }

    fn configure_mapper(mapper: &mut Mapper1, address: u16, value: u8) {
        mapper.write_prg_byte(address, 0b1000_0000);
        for i in 0..6 {
            mapper.write_prg_byte(address, (value >> i) & 1);
        }
    }

    #[test]
    fn test_set_control() {
        let mut mapper = Mapper1::new(build_cartridge_data());
        configure_mapper(&mut mapper, 0x8000, 0b01011);
        assert_eq!(mapper.control.0, 0b01011);
        assert_eq!(mapper.control.mirroring(), Mirroring::Horizontal);
        assert_eq!(mapper.control.prg_mode(), PrgMode::FixFirst);
        assert_eq!(mapper.control.chr_mode(), ChrMode::Consecutive);

        configure_mapper(&mut mapper, 0x8000, 0b10010);
        assert_eq!(mapper.control.0, 0b10010);
        assert_eq!(mapper.control.mirroring(), Mirroring::Vertical);
        assert_eq!(mapper.control.prg_mode(), PrgMode::Consecutive);
        assert_eq!(mapper.control.chr_mode(), ChrMode::NonConsecutive);
    }

    #[test]
    fn test_set_prg() {
        let mut mapper = Mapper1::new(build_cartridge_data());
        configure_mapper(&mut mapper, 0xE000, 0b01011);
        assert_eq!(mapper.prg_0, 0b01011);
        assert_eq!(mapper.chr_0, 0);
        assert_eq!(mapper.chr_1, 0);
    }

    #[test]
    fn test_set_chr_0() {
        let mut mapper = Mapper1::new(build_cartridge_data());
        configure_mapper(&mut mapper, 0xA000, 0b01010);
        assert_eq!(mapper.chr_0, 0b01010);
        assert_eq!(mapper.prg_0, 0);
        assert_eq!(mapper.chr_1, 0);
    }

    #[test]
    fn test_set_chr_1() {
        let mut mapper = Mapper1::new(build_cartridge_data());
        configure_mapper(&mut mapper, 0xC000, 0b01010);
        assert_eq!(mapper.chr_1, 0b01010);
        assert_eq!(mapper.prg_0, 0);
        assert_eq!(mapper.chr_0, 0);
    }

    #[test]
    fn test_prg_ram() {
        let mut mapper = Mapper1::new(build_cartridge_data());
        mapper.write_prg_byte(0x6001, 0xFA);
        assert_eq!(mapper.read_prg_byte(0x6001), 0xFA);
    }

    #[test]
    fn test_prg_rom() {
        let mut mapper = Mapper1::new(build_cartridge_data());
        configure_mapper(&mut mapper, 0x8000, 0b11011); // FixFirst PRG, Nonconsecutive CHR
        configure_mapper(&mut mapper, 0xE000, 3); // High addr range maps to page 3
        assert_eq!(mapper.control.prg_mode(), PrgMode::FixFirst);
        assert_eq!(mapper.prg_0, 3);

        // Test the low addr range
        mapper.data.prg_rom.data[1] = 0xFC;
        assert_eq!(mapper.read_prg_byte(0x8001), 0xFC);

        // Test the high addr range
        mapper.data.prg_rom.data[PageSize::SixteenKb as usize * 3 + 5] = 0xFB;
        assert_eq!(mapper.read_prg_byte(0xC005), 0xFB);
    }

    #[test]
    fn test_chr_rom() {
        let mut mapper = Mapper1::new(build_cartridge_data());
        configure_mapper(&mut mapper, 0x8000, 0b11011); // FixFirst PRG, Nonconsecutive CHR
        configure_mapper(&mut mapper, 0xA000, 3); // Low addr range maps to page 3
        configure_mapper(&mut mapper, 0xC000, 5); // Low addr range maps to page 5
        assert_eq!(mapper.control.chr_mode(), ChrMode::NonConsecutive);
        assert_eq!(mapper.chr_0, 3);
        assert_eq!(mapper.chr_1, 5);

        // Test the low addr range
        mapper.data.chr_rom.data[PageSize::FourKb as usize * 3 + 8] = 0xFC;
        assert_eq!(mapper.read_chr_byte(0x0008), 0xFC);

        // Test the high addr range

        mapper.data.chr_rom.data[PageSize::FourKb as usize * 5 + 9] = 0xFD;
        assert_eq!(mapper.read_chr_byte(0x1009), 0xFD);
    }

}
