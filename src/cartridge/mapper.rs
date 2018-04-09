use super::Mirroring;

pub trait Mapper {
    fn signal_scanline(&mut self) {
        // A blank placeholder is fine for most mappers
    }
    fn read_prg_byte(&self, address: u16) -> u8;
    fn write_prg_byte(&mut self, address: u16, value: u8);
    fn read_chr_byte(&self, address: u16) -> u8;
    fn write_chr_byte(&mut self, address: u16, value: u8);
    fn mirroring(&self) -> Mirroring;
    fn irq_flag(&self) -> bool {
        false
    }
}
