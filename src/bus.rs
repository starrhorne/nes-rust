use apu::Apu;
use cartridge::Cartridge;
use controller::Controller;
use ppu::Ppu;
use ppu::result::PpuResult;
use std::cell::RefCell;
use std::rc::Rc;

pub struct Interrupt {
    schedule: Option<u8>,
}

impl Interrupt {
    fn new() -> Self {
        Interrupt { schedule: None }
    }

    fn tick(&mut self) {
        match self.schedule.as_mut() {
            Some(v) => if *v > 0 {
                *v -= 1
            },
            None => (),
        };
    }

    pub fn schedule(&mut self, n: u8) {
        self.schedule = Some(n);
    }

    pub fn acknowledge(&mut self) {
        self.schedule = None;
    }

    pub fn ready(&self) -> bool {
        match self.schedule {
            Some(v) => v == 0,
            None => false,
        }
    }
}

pub struct Bus {
    pub ram: [u8; 2048],
    pub apu: Apu,
    pub ppu: Ppu,
    pub cartridge: Option<Rc<RefCell<Cartridge>>>,
    pub controller_0: Controller,
    pub controller_1: Controller,
    pub cycles: u64,
    pub nmi: Interrupt,
    pub draw: bool,
    cpu_stall_cycles: usize,
}

impl Bus {
    pub fn new() -> Self {
        Bus {
            ppu: Ppu::new(),
            apu: Apu::new(),
            cartridge: None,
            controller_0: Controller::new(),
            controller_1: Controller::new(),
            ram: [0; 2048],
            cycles: 0,
            nmi: Interrupt::new(),
            draw: false, // add: mapper/cartridge
            cpu_stall_cycles: 0,
        }
    }

    pub fn reset_cpu_stall_cycles(&mut self) -> usize {
        let c = self.cpu_stall_cycles + self.apu.dmc.reset_cpu_stall_cycles() as usize;
        self.cpu_stall_cycles = 0;
        c
    }

    // unclocked_read_byte and unclocked_write_byte are unclocked memory access
    pub fn unclocked_read_byte(&mut self, address: u16) -> u8 {
        match address {
            0...0x1FFF => self.ram[address as usize % 0x0800],
            0x2000...0x3FFF => self.ppu.read_register(address),
            0x4015 => self.apu.read_register(),
            0x4016 => self.controller_0.read_register(),
            0x4017 => self.controller_1.read_register(),
            0x4018...0xFFFF => if let Some(ref c) = self.cartridge {
                c.borrow().read_prg_byte(address)
            } else {
                (address >> 8) as u8
            },
            address => (address >> 8) as u8,
        }
    }

    fn unclocked_write_byte(&mut self, address: u16, value: u8) {
        match address {
            0...0x1FFF => self.ram[address as usize % 0x0800] = value,
            0x2000...0x3FFF => self.ppu.write_register(address, value),
            0x4000...0x4013 | 0x4015 => self.apu.write_register(address, value, self.cycles),
            0x4017 => self.apu.write_register(address, value, self.cycles),
            0x4014 => self.oam_dma(value as u16),
            0x4016 => {
                self.controller_0.write_register(value);
                self.controller_1.write_register(value);
            }

            0x4018...0xFFFF => if let Some(ref c) = self.cartridge {
                c.borrow_mut().write_prg_byte(address, value);
            },
            _ => (),
        }
    }

    fn oam_dma(&mut self, bank: u16) {
        self.cpu_stall_cycles += 513 + (self.cycles as usize % 2);
        for i in 0..256 {
            let v = self.unclocked_read_byte(bank * 0x100 + i);
            self.ppu.registers.write_oam_data(v);
        }
    }

    pub fn read_byte<T: Into<u16>>(&mut self, address: T) -> u8 {
        self.tick();
        self.unclocked_read_byte(address.into())
    }

    pub fn write_byte<T: Into<u16>>(&mut self, address: T, value: u8) {
        self.tick();
        self.unclocked_write_byte(address.into(), value)
    }

    pub fn read_noncontinuous_word<T: Into<u16>, U: Into<u16>>(&mut self, a: T, b: U) -> u16 {
        (self.read_byte(a) as u16) | (self.read_byte(b) as u16) << 8
    }

    pub fn read_word<T: Into<u16>>(&mut self, address: T) -> u16 {
        let address = address.into();
        self.read_noncontinuous_word(address, address + 1)
    }

    pub fn tick(&mut self) {
        self.cycles += 1;

        let c = self.cycles;
        self.apu.tick(c);

        self.nmi.tick();

        // Roughly 3x per frame. I made this number up.
        if c % 10_000 == 0 {
            self.ppu.tick_decay();
        }

        let r = self.ppu.tick();
        self.handle_ppu_result(r);

        let r = self.ppu.tick();
        self.handle_ppu_result(r);

        let r = self.ppu.tick();
        self.handle_ppu_result(r);
    }

    pub fn irq(&self) -> bool {
        let cartridge_irq = if let Some(ref c) = self.cartridge {
            c.borrow().irq_flag()
        } else {
            false
        };
        cartridge_irq || self.apu.irq_flag()
    }

    fn handle_ppu_result(&mut self, result: PpuResult) {
        match result {
            PpuResult::Nmi => {
                self.nmi.schedule(1);
            }
            PpuResult::Scanline => if let Some(ref c) = self.cartridge {
                c.borrow_mut().signal_scanline();
            },
            PpuResult::Draw => {
                self.draw = true;
            }
            PpuResult::None => {}
        }
    }

    pub fn load_rom_from_memory(&mut self, data: &[u8]) {
        let c = Rc::new(RefCell::new(Cartridge::new(data)));
        self.ppu.registers.vram.set_cartridge(c.clone());
        self.apu.dmc.set_cartridge(c.clone());
        self.cartridge = Some(c);
    }

    pub fn reset(&mut self) {
        self.apu.reset();
        // TODO - PPU and cartridge reset?
    }
}

#[cfg(test)]
mod test {
    use super::*;
}
