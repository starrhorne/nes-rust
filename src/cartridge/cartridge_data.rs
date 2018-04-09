use super::cartridge_header::CartridgeHeader;
use super::pager::Pager;

pub struct CartridgeData {
    pub header: CartridgeHeader,
    pub prg_rom: Pager,
    pub prg_ram: Pager,
    pub chr_rom: Pager,
    pub chr_ram: Pager,
}

impl CartridgeData {
    pub fn new(data: &[u8]) -> Self {
        let header = CartridgeHeader::new(data);

        CartridgeData {
            header: header,
            prg_rom: Pager::new(data[header.prg_rom_range()].to_vec()),
            chr_rom: Pager::new(data[header.chr_rom_range()].to_vec()),
            prg_ram: Pager::new(vec![0u8; header.prg_ram_bytes()]),
            chr_ram: Pager::new(vec![0u8; header.chr_ram_bytes()]),
        }
    }
}
