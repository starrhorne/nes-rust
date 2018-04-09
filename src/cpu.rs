use bus::Bus;
use std::fmt::Write;

use cpu_debug::{INSTRUCTION_NAMES, INSTRUCTION_SIZES};

#[cfg_attr(rustfmt, rustfmt_skip)]
enum Flag {
    Carry      = 0b00000001,
    Zero       = 0b00000010,
    IrqDisable = 0b00000100,
    Decimal    = 0b00001000,
    Break      = 0b00010000,
    Push       = 0b00100000,
    Overflow   = 0b01000000,
    Negative   = 0b10000000,
}

#[allow(dead_code)]
#[derive(Debug, Copy, Clone, PartialEq)]
enum Mode {
    Immediate,
    ZeroPage,
    ZeroPageX,
    ZeroPageY,
    Absolute,
    AbsoluteX,
    AbsoluteXForceTick,
    AbsoluteYForceTick,
    AbsoluteY,
    Indirect,
    IndirectX,
    IndirectY,
    IndirectYForceTick,
    NoMode,
}

#[derive(Debug, Copy, Clone, PartialEq)]
enum Interrupt {
    Nmi,
    Reset,
    Irq,
    Break,
}

pub struct Cpu {
    pub bus: Bus,
    pc: u16,
    sp: u8,
    a: u8,
    x: u8,
    y: u8,
    p: u8,
}

impl Cpu {
    pub fn new(bus: Bus) -> Self {
        Cpu {
            bus,
            pc: 0,
            sp: 0,
            a: 0,
            x: 0,
            y: 0,
            p: 0,
        }
    }

    pub fn reset(&mut self) {
        self.sp = 0xFF;
        self.p = 0x34;
        self.interrupt(Interrupt::Reset);
    }

    // Push / Pop are for the stack
    fn pop_byte(&mut self) -> u8 {
        self.sp = self.sp.wrapping_add(1);
        let address = 0x100 + self.sp as u16;
        self.bus.read_byte(address)
    }

    fn push_byte(&mut self, value: u8) {
        let address = 0x100 + self.sp as u16;
        self.bus.write_byte(address, value);
        self.sp = self.sp.wrapping_sub(1);
    }

    fn push_word(&mut self, value: u16) {
        self.push_byte((value >> 8) as u8);
        self.push_byte(value as u8);
    }

    fn pop_word(&mut self) -> u16 {
        (self.pop_byte() as u16) | ((self.pop_byte() as u16) << 8)
    }

    fn increment_pc(&mut self) {
        self.pc = self.pc.wrapping_add(1);
    }

    fn next_byte(&mut self) -> u8 {
        let original_pc = self.pc;
        self.increment_pc();
        self.bus.read_byte(original_pc)
    }

    fn next_word(&mut self) -> u16 {
        let original_pc = self.pc;
        self.increment_pc();
        self.increment_pc();
        self.bus.read_word(original_pc)
    }

    // Flags
    fn get_flag(&self, flag: Flag) -> bool {
        self.p & (flag as u8) != 0
    }

    fn set_flag(&mut self, flag: Flag, value: bool) {
        if value {
            self.p |= flag as u8;
        } else {
            self.p &= !(flag as u8);
        }
    }

    fn set_flags_zero_negative(&mut self, value: u8) {
        self.set_flag(Flag::Zero, value == 0);
        self.set_flag(Flag::Negative, (value & 0b10000000) != 0);
    }

    fn set_flags_carry_overflow(&mut self, m: u8, n: u8, result: u16) {
        self.set_flag(Flag::Carry, result > 0xFF);
        let r = result as u8;
        self.set_flag(Flag::Overflow, (m ^ r) & (n ^ r) & 0x80 != 0);
    }

    // Flags
    fn carry(&self) -> u8 {
        self.p & (Flag::Carry as u8)
    }

    // Assumes that the PC is on the first byte after the opcode
    fn operand_address(&mut self, mode: Mode) -> u16 {
        match mode {
            Mode::Immediate => {
                let original_pc = self.pc;
                self.increment_pc();
                original_pc
            }
            Mode::ZeroPage => self.next_byte() as u16,
            Mode::ZeroPageX => {
                self.bus.tick();
                low_byte(offset(self.next_byte(), self.x))
            }
            Mode::ZeroPageY => {
                self.bus.tick();
                low_byte(offset(self.next_byte(), self.y))
            }
            Mode::Absolute => self.next_word(),
            Mode::AbsoluteX => {
                let base = self.next_word();
                if cross(base, self.x) {
                    self.bus.tick();
                };
                offset(base, self.x)
            }
            Mode::AbsoluteXForceTick => {
                self.bus.tick();
                offset(self.next_word(), self.x)
            }
            Mode::AbsoluteY => {
                let base = self.next_word();
                if cross(base, self.y) {
                    self.bus.tick();
                }
                offset(base, self.y)
            }
            Mode::AbsoluteYForceTick => {
                self.bus.tick();
                offset(self.next_word(), self.y)
            }
            Mode::Indirect => {
                let i = self.next_word();
                self.bus
                    .read_noncontinuous_word(i, high_byte(i) | low_byte(i + 1))
            }
            Mode::IndirectX => {
                self.bus.tick();
                let i = offset(self.next_byte(), self.x);
                self.bus
                    .read_noncontinuous_word(low_byte(i), low_byte(i + 1))
            }
            Mode::IndirectY => {
                let i = self.next_byte();
                let base = self.bus.read_noncontinuous_word(i, low_byte(i + 1));
                if cross(base, self.y) {
                    self.bus.tick();
                }
                offset(base, self.y)
            }
            Mode::IndirectYForceTick => {
                let i = self.next_byte();
                let base = self.bus.read_noncontinuous_word(i, low_byte(i + 1));
                self.bus.tick();
                offset(base, self.y)
            }
            Mode::NoMode => panic!("Mode::NoMode should never be used to read from memory"),
        }
    }

    fn read_operand(&mut self, mode: Mode) -> u8 {
        let address = self.operand_address(mode);
        self.bus.read_byte(address)
    }

    fn interrupt(&mut self, kind: Interrupt) {
        let (ticks, push, address, flags) = match kind {
            Interrupt::Nmi => (2, true, 0xFFFAu16, vec![Flag::IrqDisable]),
            Interrupt::Reset => (5, false, 0xFFFCu16, vec![]),
            Interrupt::Irq => (2, true, 0xFFFEu16, vec![Flag::IrqDisable]),
            Interrupt::Break => (1, true, 0xFFFEu16, vec![Flag::IrqDisable]),
        };

        for _ in 0..ticks {
            self.bus.tick();
        }

        if push {
            let pc = self.pc;
            // The Break and Push flags only exist in registers pushed to
            // the stack, never in the actual P register.
            let mut p = self.p | Flag::Push as u8;
            if let Interrupt::Break = kind {
                p |= Flag::Break as u8;
            }
            self.push_word(pc);
            self.push_byte(p);
        }

        for f in flags {
            self.set_flag(f, true);
        }

        self.pc = self.bus.read_word(address);
    }

    #[allow(dead_code)]
    pub fn log_next_instruction(&mut self) {
        let pc = self.pc;
        let rom_offset = 15 + (self.pc % 0x4000);
        let opcode = self.bus.unclocked_read_byte(pc) as usize;
        let mut args = String::new();
        for i in 1..INSTRUCTION_SIZES[opcode] {
            write!(&mut args, "{:02X} ", self.bus.unclocked_read_byte(pc + i)).expect("it to work");
        }
        println!(
            "OFFSET:{:06x}\tPC:{:04x}\tA:{:02x}\tX:{:02x}\tY:{:02x}\tP:{:08b}\tTEST:{:02x}\t[{:02x}] {}\t{}",
            rom_offset,
            pc,
            self.a,
            self.x,
            self.y,
            self.p,
            self.bus.unclocked_read_byte(0x6000),
            opcode,
            INSTRUCTION_NAMES[opcode as usize],
            args,
        );
    }

    pub fn execute_next_instruction(&mut self) {
        if self.bus.nmi.ready() {
            self.bus.nmi.acknowledge();
            self.interrupt(Interrupt::Nmi)
        } else if self.bus.irq() && !self.get_flag(Flag::IrqDisable) {
            self.interrupt(Interrupt::Irq)
        }

        #[cfg(feature = "log")]
        self.log_next_instruction();

        let instruction = self.next_byte();
        self.execute_instruction(instruction);
    }

    fn execute_instruction(&mut self, opcode: u8) {
        match opcode {
            // Loads
            0xa1 => self.lda(Mode::IndirectX),
            0xa5 => self.lda(Mode::ZeroPage),
            0xa9 => self.lda(Mode::Immediate),
            0xad => self.lda(Mode::Absolute),
            0xb1 => self.lda(Mode::IndirectY),
            0xb5 => self.lda(Mode::ZeroPageX),
            0xb9 => self.lda(Mode::AbsoluteY),
            0xbd => self.lda(Mode::AbsoluteX),

            0xa2 => self.ldx(Mode::Immediate),
            0xa6 => self.ldx(Mode::ZeroPage),
            0xb6 => self.ldx(Mode::ZeroPageY),
            0xae => self.ldx(Mode::Absolute),
            0xbe => self.ldx(Mode::AbsoluteY),

            0xa0 => self.ldy(Mode::Immediate),
            0xa4 => self.ldy(Mode::ZeroPage),
            0xb4 => self.ldy(Mode::ZeroPageX),
            0xac => self.ldy(Mode::Absolute),
            0xbc => self.ldy(Mode::AbsoluteX),

            // Stores
            0x85 => self.sta(Mode::ZeroPage),
            0x95 => self.sta(Mode::ZeroPageX),
            0x8d => self.sta(Mode::Absolute),
            0x9d => self.sta(Mode::AbsoluteXForceTick),
            0x99 => self.sta(Mode::AbsoluteYForceTick),
            0x81 => self.sta(Mode::IndirectX),
            0x91 => self.sta(Mode::IndirectYForceTick),

            0x86 => self.stx(Mode::ZeroPage),
            0x96 => self.stx(Mode::ZeroPageY),
            0x8e => self.stx(Mode::Absolute),

            0x84 => self.sty(Mode::ZeroPage),
            0x94 => self.sty(Mode::ZeroPageX),
            0x8c => self.sty(Mode::Absolute),

            // Arithmetic
            0x69 => self.adc(Mode::Immediate),
            0x65 => self.adc(Mode::ZeroPage),
            0x75 => self.adc(Mode::ZeroPageX),
            0x6d => self.adc(Mode::Absolute),
            0x7d => self.adc(Mode::AbsoluteX),
            0x79 => self.adc(Mode::AbsoluteY),
            0x61 => self.adc(Mode::IndirectX),
            0x71 => self.adc(Mode::IndirectY),

            0xe9 => self.sbc(Mode::Immediate),
            0xe5 => self.sbc(Mode::ZeroPage),
            0xf5 => self.sbc(Mode::ZeroPageX),
            0xed => self.sbc(Mode::Absolute),
            0xfd => self.sbc(Mode::AbsoluteX),
            0xf9 => self.sbc(Mode::AbsoluteY),
            0xe1 => self.sbc(Mode::IndirectX),
            0xf1 => self.sbc(Mode::IndirectY),

            // Comparisons
            0xc9 => self.cmp(Mode::Immediate),
            0xc5 => self.cmp(Mode::ZeroPage),
            0xd5 => self.cmp(Mode::ZeroPageX),
            0xcd => self.cmp(Mode::Absolute),
            0xdd => self.cmp(Mode::AbsoluteX),
            0xd9 => self.cmp(Mode::AbsoluteY),
            0xc1 => self.cmp(Mode::IndirectX),
            0xd1 => self.cmp(Mode::IndirectY),

            0xe0 => self.cpx(Mode::Immediate),
            0xe4 => self.cpx(Mode::ZeroPage),
            0xec => self.cpx(Mode::Absolute),

            0xc0 => self.cpy(Mode::Immediate),
            0xc4 => self.cpy(Mode::ZeroPage),
            0xcc => self.cpy(Mode::Absolute),

            // Bitwise operations
            0x29 => self.and(Mode::Immediate),
            0x25 => self.and(Mode::ZeroPage),
            0x35 => self.and(Mode::ZeroPageX),
            0x2d => self.and(Mode::Absolute),
            0x3d => self.and(Mode::AbsoluteX),
            0x39 => self.and(Mode::AbsoluteY),
            0x21 => self.and(Mode::IndirectX),
            0x31 => self.and(Mode::IndirectY),

            0x09 => self.ora(Mode::Immediate),
            0x05 => self.ora(Mode::ZeroPage),
            0x15 => self.ora(Mode::ZeroPageX),
            0x0d => self.ora(Mode::Absolute),
            0x1d => self.ora(Mode::AbsoluteX),
            0x19 => self.ora(Mode::AbsoluteY),
            0x01 => self.ora(Mode::IndirectX),
            0x11 => self.ora(Mode::IndirectY),

            0x49 => self.eor(Mode::Immediate),
            0x45 => self.eor(Mode::ZeroPage),
            0x55 => self.eor(Mode::ZeroPageX),
            0x4d => self.eor(Mode::Absolute),
            0x5d => self.eor(Mode::AbsoluteX),
            0x59 => self.eor(Mode::AbsoluteY),
            0x41 => self.eor(Mode::IndirectX),
            0x51 => self.eor(Mode::IndirectY),

            0x24 => self.bit(Mode::ZeroPage),
            0x2c => self.bit(Mode::Absolute),

            // Shifts and rotates
            0x2a => self.rol_a(),
            0x26 => self.rol(Mode::ZeroPage),
            0x36 => self.rol(Mode::ZeroPageX),
            0x2e => self.rol(Mode::Absolute),
            0x3e => self.rol(Mode::AbsoluteXForceTick),

            0x6a => self.ror_a(),
            0x66 => self.ror(Mode::ZeroPage),
            0x76 => self.ror(Mode::ZeroPageX),
            0x6e => self.ror(Mode::Absolute),
            0x7e => self.ror(Mode::AbsoluteXForceTick),

            0x0a => self.asl_a(),
            0x06 => self.asl(Mode::ZeroPage),
            0x16 => self.asl(Mode::ZeroPageX),
            0x0e => self.asl(Mode::Absolute),
            0x1e => self.asl(Mode::AbsoluteXForceTick),

            0x4a => self.lsr_a(),
            0x46 => self.lsr(Mode::ZeroPage),
            0x56 => self.lsr(Mode::ZeroPageX),
            0x4e => self.lsr(Mode::Absolute),
            0x5e => self.lsr(Mode::AbsoluteXForceTick),

            // Increments and decrements
            0xe6 => self.inc(Mode::ZeroPage),
            0xf6 => self.inc(Mode::ZeroPageX),
            0xee => self.inc(Mode::Absolute),
            0xfe => self.inc(Mode::AbsoluteXForceTick),

            0xc6 => self.dec(Mode::ZeroPage),
            0xd6 => self.dec(Mode::ZeroPageX),
            0xce => self.dec(Mode::Absolute),
            0xde => self.dec(Mode::AbsoluteXForceTick),

            0xe8 => self.inx(),
            0xca => self.dex(),
            0xc8 => self.iny(),
            0x88 => self.dey(),

            // Register moves
            0xaa => self.tax(),
            0xa8 => self.tay(),
            0x8a => self.txa(),
            0x98 => self.tya(),
            0x9a => self.txs(),
            0xba => self.tsx(),

            // Flag operations
            0x18 => self.clc(),
            0x38 => self.sec(),
            0x58 => self.cli(),
            0x78 => self.sei(),
            0xb8 => self.clv(),
            0xd8 => self.cld(),
            0xf8 => self.sed(),

            // Branches
            0x10 => self.bpl(),
            0x30 => self.bmi(),
            0x50 => self.bvc(),
            0x70 => self.bvs(),
            0x90 => self.bcc(),
            0xb0 => self.bcs(),
            0xd0 => self.bne(),
            0xf0 => self.beq(),

            // Jumps
            0x4c => self.jmp(Mode::Absolute),
            0x6c => self.jmp(Mode::Indirect),

            // Procedure calls
            0x20 => self.jsr(),
            0x60 => self.rts(),
            0x00 => self.brk(),
            0x40 => self.rti(),

            // Stack operations
            0x48 => self.pha(),
            0x68 => self.pla(),
            0x08 => self.php(),
            0x28 => self.plp(),

            // No operation
            0xea => self.nop(),

            // Undocumented Operations
            0x1A | 0x3A | 0x5A | 0x7A | 0xDA | 0xFA => self.nop(),

            0x0C => self.nop_read(Mode::Absolute),

            0x1C | 0x3C | 0x5C | 0x7C | 0xDC | 0xFC => self.nop_read(Mode::AbsoluteX),
            0x04 | 0x44 | 0x64 => self.nop_read(Mode::ZeroPage),
            0x14 | 0x34 | 0x54 | 0x74 | 0xd4 | 0xf4 => self.nop_read(Mode::ZeroPageX),
            0x80 | 0x82 | 0x89 | 0xc2 | 0xe2 => self.nop_read(Mode::Immediate),

            0x07 => self.slo(Mode::ZeroPage),
            0x17 => self.slo(Mode::ZeroPageX),
            0x03 => self.slo(Mode::IndirectX),
            0x13 => self.slo(Mode::IndirectY),
            0x0F => self.slo(Mode::Absolute),
            0x1F => self.slo(Mode::AbsoluteX),
            0x1B => self.slo(Mode::AbsoluteY),

            0x27 => self.rla(Mode::ZeroPage),
            0x37 => self.rla(Mode::ZeroPageX),
            0x23 => self.rla(Mode::IndirectX),
            0x33 => self.rla(Mode::IndirectY),
            0x2F => self.rla(Mode::Absolute),
            0x3F => self.rla(Mode::AbsoluteX),
            0x3B => self.rla(Mode::AbsoluteY),

            0x47 => self.sre(Mode::ZeroPage),
            0x57 => self.sre(Mode::ZeroPageX),
            0x43 => self.sre(Mode::IndirectX),
            0x53 => self.sre(Mode::IndirectY),
            0x4F => self.sre(Mode::Absolute),
            0x5F => self.sre(Mode::AbsoluteX),
            0x5B => self.sre(Mode::AbsoluteY),

            0x67 => self.rra(Mode::ZeroPage),
            0x77 => self.rra(Mode::ZeroPageX),
            0x63 => self.rra(Mode::IndirectX),
            0x73 => self.rra(Mode::IndirectY),
            0x6F => self.rra(Mode::Absolute),
            0x7F => self.rra(Mode::AbsoluteX),
            0x7B => self.rra(Mode::AbsoluteY),

            0x87 => self.sax(Mode::ZeroPage),
            0x97 => self.sax(Mode::ZeroPageY),
            0x83 => self.sax(Mode::IndirectX),
            0x8F => self.sax(Mode::Absolute),

            0xA7 => self.lax(Mode::ZeroPage),
            0xB7 => self.lax(Mode::ZeroPageY),
            0xA3 => self.lax(Mode::IndirectX),
            0xB3 => self.lax(Mode::IndirectY),
            0xAF => self.lax(Mode::Absolute),
            0xBF => self.lax(Mode::AbsoluteY),

            0xC7 => self.dcp(Mode::ZeroPage),
            0xD7 => self.dcp(Mode::ZeroPageX),
            0xC3 => self.dcp(Mode::IndirectX),
            0xD3 => self.dcp(Mode::IndirectY),
            0xCF => self.dcp(Mode::Absolute),
            0xDF => self.dcp(Mode::AbsoluteX),
            0xDB => self.dcp(Mode::AbsoluteY),

            0xE7 => self.isc(Mode::ZeroPage),
            0xF7 => self.isc(Mode::ZeroPageX),
            0xE3 => self.isc(Mode::IndirectX),
            0xF3 => self.isc(Mode::IndirectY),
            0xEF => self.isc(Mode::Absolute),
            0xFF => self.isc(Mode::AbsoluteX),
            0xFB => self.isc(Mode::AbsoluteY),

            0x0B | 0x2B => self.anc(),
            0x4B => self.alr(),
            0x6B => self.arr(),
            0x8B => self.xaa(),
            0xAB => self.lxa(),
            0xCB => self.axs(),
            0xEB => self.sbc_nop(),
            0x93 => self.ahx(Mode::IndirectY),
            0x9F => self.ahx(Mode::AbsoluteY),
            0x9c => self.shy(),
            0x9e => self.shx(),
            0x9b => self.tas(Mode::AbsoluteY),
            0xbb => self.las(Mode::AbsoluteY),

            0x02 => println!("----------------PING----------------------"),

            _ => panic!("unimplemented or illegal instruction: 0x{:X}", opcode),
        }
    }

    // Loads
    fn lda(&mut self, mode: Mode) {
        let operand = self.read_operand(mode);
        self.set_flags_zero_negative(operand);
        self.a = operand;
    }

    fn ldx(&mut self, mode: Mode) {
        let operand = self.read_operand(mode);
        self.set_flags_zero_negative(operand);
        self.x = operand;
    }

    fn ldy(&mut self, mode: Mode) {
        let operand = self.read_operand(mode);
        self.set_flags_zero_negative(operand);
        self.y = operand;
    }

    // Stores
    fn sta(&mut self, mode: Mode) {
        let address = self.operand_address(mode);
        let value = self.a;
        self.bus.write_byte(address, value);
    }

    fn stx(&mut self, mode: Mode) {
        let address = self.operand_address(mode);
        let value = self.x;
        self.bus.write_byte(address, value);
    }

    fn sty(&mut self, mode: Mode) {
        let address = self.operand_address(mode);
        let value = self.y;
        self.bus.write_byte(address, value);
    }

    // Math

    fn adc(&mut self, mode: Mode) {
        let a = self.a;
        let operand = self.read_operand(mode);
        let result = a as u16 + operand as u16 + self.carry() as u16;
        self.set_flags_carry_overflow(a, operand, result);
        self.set_flags_zero_negative(result as u8);
        self.a = result as u8;
    }

    fn sbc(&mut self, mode: Mode) {
        let a = self.a;
        let operand = !self.read_operand(mode);
        let result = a as u16 + operand as u16 + self.carry() as u16;
        self.set_flags_carry_overflow(a, operand, result);
        self.set_flags_zero_negative(result as u8);
        self.a = result as u8;
    }

    fn cmp(&mut self, mode: Mode) {
        let operand = self.read_operand(mode);
        let a = self.a;
        self.set_flags_zero_negative(a.wrapping_sub(operand));
        self.set_flag(Flag::Carry, a >= operand);
    }

    fn cpx(&mut self, mode: Mode) {
        let operand = self.read_operand(mode);
        let x = self.x;
        self.set_flags_zero_negative(x.wrapping_sub(operand));
        self.set_flag(Flag::Carry, x >= operand);
    }

    fn cpy(&mut self, mode: Mode) {
        let operand = self.read_operand(mode);
        let y = self.y;
        self.set_flags_zero_negative(y.wrapping_sub(operand));
        self.set_flag(Flag::Carry, y >= operand);
    }

    fn and(&mut self, mode: Mode) {
        let operand = self.read_operand(mode);
        let result = self.a & operand;
        self.set_flags_zero_negative(result);
        self.a = result;
    }

    fn ora(&mut self, mode: Mode) {
        let operand = self.read_operand(mode);
        let result = self.a | operand;
        self.set_flags_zero_negative(result);
        self.a = result;
    }

    fn eor(&mut self, mode: Mode) {
        let operand = self.read_operand(mode);
        let result = self.a ^ operand;
        self.set_flags_zero_negative(result);
        self.a = result;
    }

    fn bit(&mut self, mode: Mode) {
        let operand = self.read_operand(mode);
        let result = self.a & operand;
        self.set_flag(Flag::Zero, result == 0);
        self.set_flag(Flag::Overflow, operand & 0b01000000 != 0);
        self.set_flag(Flag::Negative, operand & 0b10000000 != 0);
    }

    fn rol(&mut self, mode: Mode) {
        self.do_rol(mode);
    }

    fn do_rol(&mut self, mode: Mode) -> u8 {
        let address = self.operand_address(mode);
        let operand = self.bus.read_byte(address);
        let result = (operand << 1) | self.carry();
        self.set_flag(Flag::Carry, operand & 0b10000000 != 0);
        self.bus.tick();
        self.set_flags_zero_negative(result);
        self.bus.write_byte(address, result);
        result
    }

    fn rol_a(&mut self) {
        let operand = self.a;
        let result = (operand << 1) | self.carry();
        self.set_flag(Flag::Carry, operand & 0b10000000 != 0);
        self.set_flags_zero_negative(result);
        self.a = result;
        self.bus.tick();
    }

    fn ror(&mut self, mode: Mode) {
        self.do_ror(mode);
    }

    fn do_ror(&mut self, mode: Mode) -> u8 {
        let address = self.operand_address(mode);
        let operand = self.bus.read_byte(address);
        let result = (operand >> 1) | (self.carry() << 7);
        self.set_flag(Flag::Carry, operand & 1 != 0);
        self.bus.tick();
        self.set_flags_zero_negative(result);
        self.bus.write_byte(address, result);
        result
    }

    fn ror_a(&mut self) {
        let operand = self.a;
        let result = (operand >> 1) | (self.carry() << 7);
        self.set_flag(Flag::Carry, operand & 1 != 0);
        self.set_flags_zero_negative(result);
        self.a = result;
        self.bus.tick();
    }

    fn asl(&mut self, mode: Mode) {
        self.do_asl(mode);
    }

    fn do_asl(&mut self, mode: Mode) -> u8 {
        let address = self.operand_address(mode);
        let operand = self.bus.read_byte(address);
        let result = operand << 1;
        self.set_flag(Flag::Carry, operand & 0b10000000 != 0);
        self.bus.tick();
        self.set_flags_zero_negative(result);
        self.bus.write_byte(address, result);
        result
    }

    fn asl_a(&mut self) {
        let operand = self.a;
        let result = operand << 1;
        self.set_flag(Flag::Carry, operand & 0b10000000 != 0);
        self.set_flags_zero_negative(result);
        self.a = result;
        self.bus.tick();
    }

    fn lsr(&mut self, mode: Mode) {
        self.do_lsr(mode);
    }

    fn do_lsr(&mut self, mode: Mode) -> u8 {
        let address = self.operand_address(mode);
        let operand = self.bus.read_byte(address);
        let result = operand >> 1;
        self.set_flag(Flag::Carry, operand & 1 != 0);
        self.bus.tick();
        self.set_flags_zero_negative(result);
        self.bus.write_byte(address, result);
        result
    }

    fn lsr_a(&mut self) {
        let operand = self.a;
        let result = operand >> 1;
        self.set_flag(Flag::Carry, operand & 1 != 0);
        self.set_flags_zero_negative(result);
        self.a = result;
        self.bus.tick();
    }

    fn inc(&mut self, mode: Mode) {
        self.do_inc(mode);
    }

    fn do_inc(&mut self, mode: Mode) -> u8 {
        let address = self.operand_address(mode);
        let operand = self.bus.read_byte(address);
        let result = operand.wrapping_add(1);
        self.bus.tick();
        self.set_flags_zero_negative(result);
        self.bus.write_byte(address, result);
        result
    }

    fn dec(&mut self, mode: Mode) {
        self.do_dec(mode);
    }

    fn do_dec(&mut self, mode: Mode) -> u8 {
        let address = self.operand_address(mode);
        let operand = self.bus.read_byte(address);
        let result = operand.wrapping_sub(1);
        self.bus.tick();
        self.set_flags_zero_negative(result);
        self.bus.write_byte(address, result);
        result
    }

    fn inx(&mut self) {
        let result = self.x.wrapping_add(1);
        self.bus.tick();
        self.set_flags_zero_negative(result);
        self.x = result;
    }

    fn dex(&mut self) {
        let result = self.x.wrapping_sub(1);
        self.bus.tick();
        self.set_flags_zero_negative(result);
        self.x = result;
    }

    fn iny(&mut self) {
        let result = self.y.wrapping_add(1);
        self.bus.tick();
        self.set_flags_zero_negative(result);
        self.y = result;
    }

    fn dey(&mut self) {
        let result = self.y.wrapping_sub(1);
        self.bus.tick();
        self.set_flags_zero_negative(result);
        self.y = result;
    }

    fn tax(&mut self) {
        let result = self.a;
        self.bus.tick();
        self.set_flags_zero_negative(result);
        self.x = result;
    }

    fn tay(&mut self) {
        let result = self.a;
        self.bus.tick();
        self.set_flags_zero_negative(result);
        self.y = result;
    }

    fn txa(&mut self) {
        let result = self.x;
        self.bus.tick();
        self.set_flags_zero_negative(result);
        self.a = result;
    }

    fn tya(&mut self) {
        let result = self.y;
        self.bus.tick();
        self.set_flags_zero_negative(result);
        self.a = result;
    }

    fn txs(&mut self) {
        let result = self.x;
        self.bus.tick();
        self.sp = result;
    }

    fn tsx(&mut self) {
        let result = self.sp;
        self.bus.tick();
        self.set_flags_zero_negative(result);
        self.x = result;
    }

    fn clc(&mut self) {
        self.set_flag(Flag::Carry, false);
        self.bus.tick();
    }

    fn sec(&mut self) {
        self.set_flag(Flag::Carry, true);
        self.bus.tick();
    }

    fn cli(&mut self) {
        self.set_flag(Flag::IrqDisable, false);
        self.bus.tick();
    }

    fn sei(&mut self) {
        self.set_flag(Flag::IrqDisable, true);
        self.bus.tick();
    }

    fn clv(&mut self) {
        self.set_flag(Flag::Overflow, false);
        self.bus.tick();
    }

    fn cld(&mut self) {
        self.set_flag(Flag::Decimal, false);
        self.bus.tick();
    }

    fn sed(&mut self) {
        self.set_flag(Flag::Decimal, true);
        self.bus.tick();
    }

    fn branch(&mut self, condition: bool) {
        // The operand is a signed 8-bit value. The `as i8 as u16` is there
        // to create a 16-bit twos-compliment representation of the operand.
        // Just casting directly to u16 doesn't work.
        let offset = self.read_operand(Mode::Immediate) as i8 as u16;

        if condition {
            self.bus.tick();

            let new_pc = self.pc.wrapping_add(offset);

            // An extra tick for page crosses.
            if high_byte(self.pc) != high_byte(new_pc) {
                self.bus.tick();
            }

            self.pc = new_pc;
        }
    }

    fn bpl(&mut self) {
        let negative = self.get_flag(Flag::Negative);
        self.branch(!negative);
    }

    fn bmi(&mut self) {
        let negative = self.get_flag(Flag::Negative);
        self.branch(negative);
    }

    fn bvc(&mut self) {
        let overflow = self.get_flag(Flag::Overflow);
        self.branch(!overflow);
    }

    fn bvs(&mut self) {
        let overflow = self.get_flag(Flag::Overflow);
        self.branch(overflow);
    }

    fn bcc(&mut self) {
        let carry = self.get_flag(Flag::Carry);
        self.branch(!carry);
    }

    fn bcs(&mut self) {
        let carry = self.get_flag(Flag::Carry);
        self.branch(carry);
    }

    fn bne(&mut self) {
        let zero = self.get_flag(Flag::Zero);
        self.branch(!zero);
    }

    fn beq(&mut self) {
        let zero = self.get_flag(Flag::Zero);
        self.branch(zero);
    }

    fn jmp(&mut self, mode: Mode) {
        self.pc = self.operand_address(mode);
    }

    fn jsr(&mut self) {
        let target_address = self.operand_address(Mode::Absolute);
        let return_address = self.pc - 1;
        self.bus.tick();
        self.push_word(return_address);
        self.pc = target_address;
    }

    fn rts(&mut self) {
        self.bus.tick();
        self.bus.tick();
        self.pc = self.pop_word() + 1;
        self.bus.tick();
    }

    fn brk(&mut self) {
        self.pc += 1;
        self.interrupt(Interrupt::Break);
    }

    fn rti(&mut self) {
        self.p = self.pop_byte();
        self.pc = self.pop_word();
    }

    fn pha(&mut self) {
        self.bus.tick();
        let a = self.a;
        self.push_byte(a);
    }

    fn pla(&mut self) {
        self.bus.tick();
        self.bus.tick();
        let result = self.pop_byte();
        self.set_flags_zero_negative(result);
        self.a = result;
    }

    fn php(&mut self) {
        self.bus.tick();
        // See http://wiki.nesdev.com/w/index.php/CPU_status_flag_behavior
        let p = self.p | Flag::Push as u8 | Flag::Break as u8;
        self.push_byte(p);
    }

    fn plp(&mut self) {
        self.bus.tick();
        self.bus.tick();
        // Push and break flags are never set in the actual P register.
        self.p = self.pop_byte() & !(Flag::Push as u8 | Flag::Break as u8);
    }

    fn nop(&mut self) {
        self.bus.tick();
    }

    fn slo(&mut self, mode: Mode) {
        let result = self.a | self.do_asl(mode);
        self.set_flags_zero_negative(result);
        self.a = result;
    }

    fn rla(&mut self, mode: Mode) {
        let result = self.a & self.do_rol(mode);
        self.set_flags_zero_negative(result);
        self.a = result;
    }

    fn sre(&mut self, mode: Mode) {
        let result = self.a ^ self.do_lsr(mode);
        self.set_flags_zero_negative(result);
        self.a = result;
    }

    fn rra(&mut self, mode: Mode) {
        let a = self.a;
        let operand = self.do_ror(mode);
        let result = a as u16 + operand as u16 + self.carry() as u16;
        self.set_flags_carry_overflow(a, operand, result);
        self.set_flags_zero_negative(result as u8);
        self.a = result as u8;
    }

    fn sax(&mut self, mode: Mode) {
        let address = self.operand_address(mode);
        let result = self.a & self.x;
        self.bus.write_byte(address, result);
    }

    fn lax(&mut self, mode: Mode) {
        self.lda(mode);
        self.x = self.a;
    }

    fn dcp(&mut self, mode: Mode) {
        let result = self.do_dec(mode);
        let a = self.a;
        self.set_flags_zero_negative(a.wrapping_sub(result));
        self.set_flag(Flag::Carry, a >= result);
    }

    fn isc(&mut self, mode: Mode) {
        let operand = !self.do_inc(mode);
        let a = self.a;
        let result = a as u16 + operand as u16 + self.carry() as u16;
        self.set_flags_carry_overflow(a, operand, result);
        self.set_flags_zero_negative(result as u8);
        self.a = result as u8;
    }

    fn anc(&mut self) {
        let operand = self.read_operand(Mode::Immediate);
        let result = self.a & operand;
        self.set_flags_zero_negative(result);
        self.set_flag(Flag::Carry, result & 0b1000_0000 != 0);
        self.a = result;
    }

    fn alr(&mut self) {
        let operand = self.read_operand(Mode::Immediate);
        let mut result = self.a & operand;
        self.set_flag(Flag::Carry, result & 1 == 1);
        result >>= 1;
        self.set_flags_zero_negative(result);
        self.a = result;
    }

    fn arr(&mut self) {
        let operand = self.read_operand(Mode::Immediate);
        let result = ((self.a & operand) >> 1) | (self.carry() << 7);

        let bit_6 = (result >> 6) & 1;
        let bit_5 = (result >> 5) & 1;
        self.set_flag(Flag::Carry, bit_6 == 1);
        self.set_flag(Flag::Overflow, bit_6 ^ bit_5 == 1);
        self.set_flags_zero_negative(result);

        self.a = result;
    }

    fn xaa(&mut self) {
        self.txa();
        self.and(Mode::Immediate);
    }

    fn lxa(&mut self) {
        self.lda(Mode::Immediate);
        self.tax();
    }

    fn axs(&mut self) {
        let a = self.a;
        let x = self.x;
        let operand = self.read_operand(Mode::Immediate);
        let result = (a & x).wrapping_sub(operand);
        self.set_flag(Flag::Carry, (a & x) >= operand);
        self.set_flags_zero_negative(result);
        self.x = result as u8;
    }

    fn sbc_nop(&mut self) {
        self.sbc(Mode::Immediate);
        self.nop();
    }

    fn ahx(&mut self, mode: Mode) {
        let address = self.operand_address(mode);
        let result = self.a & self.x & (address >> 8) as u8;
        self.bus.write_byte(address, result);
    }

    fn shx(&mut self) {
        let mut address = self.operand_address(Mode::AbsoluteY);
        if cross(address - self.y as u16, self.y) {
            address &= (self.x as u16) << 8;
        }
        let result = self.x & ((address >> 8) as u8 + 1);
        self.bus.write_byte(address, result);
    }

    fn shy(&mut self) {
        let mut address = self.operand_address(Mode::AbsoluteX);
        if cross(address - self.x as u16, self.x) {
            address &= (self.y as u16) << 8;
        }
        let result = self.y & ((address >> 8) as u8 + 1);
        self.bus.write_byte(address, result);
    }

    fn tas(&mut self, mode: Mode) {
        let address = self.operand_address(mode);
        self.sp = self.x & self.a;
        let result = self.sp & ((address >> 8) as u8 + 1);
        self.bus.write_byte(address, result);
    }
    fn las(&mut self, mode: Mode) {
        let result = self.read_operand(mode) & self.sp;
        self.a = result;
        self.x = result;
        self.sp = result;
        self.set_flags_zero_negative(result);
    }

    fn nop_read(&mut self, mode: Mode) {
        self.read_operand(mode);
    }
}

fn cross(base: u16, offset: u8) -> bool {
    high_byte(base + offset as u16) != high_byte(base)
}

fn offset<T: Into<u16>>(base: T, offset: u8) -> u16 {
    base.into() + offset as u16
}

fn low_byte<T: Into<u16>>(value: T) -> u16 {
    value.into() & 0xFF
}

fn high_byte(value: u16) -> u16 {
    value & 0xFF00
}

// fn bytes_to_word(low: u8, high: u8) -> u16 {
//     (low as u16) | (high as u16) << 8
// }

#[cfg(test)]
#[path = "./cpu_test.rs"]
mod cpu_test;
