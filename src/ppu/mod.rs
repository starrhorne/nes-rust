mod address;
mod control;
mod colors;
mod mask;
mod sprite;
mod status;
pub mod vram;
mod renderer;
mod registers;
pub mod result;

use self::registers::Registers;
use self::renderer::Renderer;
use self::result::PpuResult;

pub struct Ppu {
    pub registers: Registers,
    pub renderer: Renderer,
}

impl Ppu {
    pub fn new() -> Self {
        let mut p = Ppu {
            registers: Registers::new(),
            renderer: Renderer::new(),
        };
        p.reset();
        p
    }

    pub fn tick(&mut self) -> PpuResult {
        let regs = &mut self.registers;
        let r = self.renderer.tick(regs);
        self.renderer.step();
        r
    }

    pub fn tick_decay(&mut self) {
        self.registers.tick_decay();
    }

    pub fn reset(&mut self) {
        self.registers.reset();
        self.renderer.reset();
    }

    pub fn write_register(&mut self, address: u16, value: u8) {
        self.registers.write_register(address, value);
    }

    pub fn read_register(&mut self, address: u16) -> u8 {
        self.registers.read_register(address)
    }
}

fn nth_bit<T: Into<u16>, U: Into<u16>>(x: T, n: U) -> u8 {
    ((x.into() >> n.into()) & 1) as u8
}
