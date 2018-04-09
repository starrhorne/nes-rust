use super::*;
use super::Mode::*;
use apu::Apu;

use cpu::Cpu;
use ppu::Ppu;

macro_rules! build_cpu {
    ($bytes:expr) => {
        {
            let mut rom = vec![
                    0x4e,
                    0x45,
                    0x53,
                    0x1a,
                    0x02, // Two pages of PRG-ROM
                    0x00, // Zero pages CHR-ROM means use CHR-RAM
                    0x01, // Vertical mirroring
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
            rom.extend_from_slice(&[0u8; 2 * 0x4000]);
            let mut bus = Bus::new();
            bus.load_rom_from_memory(&rom);
            let mut cpu = Cpu::new(bus);
            cpu.pc = 0;
            let bytes = $bytes;
            for (i, &b) in bytes.iter().enumerate() {
                cpu.bus.ram[i] = b as u8;
            }
            cpu
        }
    }
}
macro_rules! build_cpu_and_run {
    ($instruction:expr, $mode:ident, $bytes:expr) => {
        {
            let op = opcode($instruction, $mode);
            let mut mem = $bytes;
            mem.insert(0, op.code);
            let mut cpu = build_cpu!(mem);
            let start_pc = cpu.pc;
            let start_cycles = cpu.bus.cycles;
            let start_p = cpu.p;
            cpu.execute_next_instruction();
            assert_eq!(0, cpu.p & start_p & !op.mask);
            assert_eq!(op.size, cpu.pc - start_pc);
            assert_eq!(op.cycles, cpu.bus.cycles - start_cycles);
            cpu
        }
    }
}

macro_rules! test_op {
    ($instruction:expr, $mode:ident, [$($b:expr),*]{$($sk:ident : $sv:expr),*} => [$($rb:expr),*]{$($ek:ident : $ev:expr),*}) => {
        {
            let op = opcode($instruction, $mode);
            let mut mem = Vec::new();
            $(mem.push($b);)*
            mem.insert(0, op.code);
            let mut cpu = build_cpu!(mem);
            let start_pc = cpu.pc;
            let start_cycles = cpu.bus.cycles;
            let start_p = cpu.p;
            $(cpu.$sk=$sv;)*
            cpu.execute_next_instruction();
            assert!(0 == cpu.p & start_p & !op.mask, "Register mask not respected. P: 0b{:b}", cpu.p);
            if op.size > 0 {
                assert!(op.size == (cpu.pc - start_pc), "Invalid instruction size. Expected: {} bytes, Got: {}", op.size, cpu.pc - start_pc);
            }
            if op.cycles > 0 {
                assert!(op.cycles == (cpu.bus.cycles - start_cycles), "Invalid instruction duration. Expected: {} cycles, Got: {}", op.cycles, cpu.bus.cycles - start_cycles);
            }
            $(
                assert!(cpu.$ek==$ev, "Incorrect Register. Expected cpu.{} to be {}, got {}", stringify!($ek), stringify!($ev), cpu.$ek);
            )*
            let mut mem = Vec::new();
            $(mem.push($rb);)*
            mem.insert(0, op.code);
            for (i, &b) in mem.iter().enumerate() {
                assert!(cpu.bus.ram[i]==b, "Incorrect Memory. Expected ram[{}] to be {}, got {}", i, b, cpu.bus.ram[i]);
            }

            cpu
        }
    }
}

#[test]
fn test_lda() {
    test_op!("lda", Immediate, [0x00]{} => []{ a: 0x00, p: 0b00000010 });
    test_op!("lda", Immediate, [0xFF]{} => []{ a: 0xFF, p: 0b10000000 });
    test_op!("lda", Immediate, [0x20]{} => []{ a: 0x20, p: 0 });
    test_op!("lda", ZeroPage,  [0x02, 0x90]{} => []{ a: 0x90 });
    test_op!("lda", ZeroPageX, [0x02, 0, 0x90]{x:1} => []{ a: 0x90 });
    test_op!("lda", Absolute,  [0x04, 0, 0, 0x90]{x:1} => []{ a: 0x90 });
    test_op!("lda", AbsoluteX, [0x03, 0, 0, 0x90]{x:1} => []{ a: 0x90 });
    test_op!("lda", AbsoluteY, [0x03, 0, 0, 0x90]{y:1} => []{ a: 0x90 });
    test_op!("lda", IndirectX, [0x02, 0, 0x05, 0, 0x90]{x:1} => []{ a: 0x90 });
    test_op!("lda", IndirectY, [0x02, 0x04, 0, 0, 0x90]{y:1} => []{ a: 0x90 });
}

#[test]
fn test_ldx() {
    test_op!("ldx", Immediate, [0x00]{}                 => []{ x: 0x00, p: 0b00000010 });
    test_op!("ldx", Immediate, [0xFF]{}                 => []{ x: 0xFF, p: 0b10000000 });
    test_op!("ldx", Immediate, [0x20]{}                 => []{ x: 0x20, p: 0 });
    test_op!("ldx", ZeroPage,  [0x02, 0x90]{}           => []{ x: 0x90 });
    test_op!("ldx", ZeroPageY, [0x02, 0, 0x90]{y:1}     => []{ x: 0x90 });
    test_op!("ldx", Absolute,  [0x04, 0, 0, 0x90]{}     => []{ x: 0x90 });
    test_op!("ldx", AbsoluteY, [0x03, 0, 0, 0x90]{y:1}  => []{ x: 0x90 });
}

#[test]
fn test_ldy() {
    test_op!("ldy", Immediate, [0x00]{} => []{ y: 0x00, p: 0b00000010 });
    test_op!("ldy", Immediate, [0xFF]{} => []{ y: 0xFF, p: 0b10000000 });
    test_op!("ldy", Immediate, [0x20]{} => []{ y: 0x20, p: 0 });
    test_op!("ldy", ZeroPage,  [0x02, 0x90]{} => []{ y: 0x90 });
    test_op!("ldy", ZeroPageX, [0x02, 0, 0x90]{x:1} => []{ y: 0x90 });
    test_op!("ldy", Absolute,  [0x04, 0, 0, 0x90]{x:1} => []{ y: 0x90 });
    test_op!("ldy", AbsoluteX, [0x03, 0, 0, 0x90]{x:1} => []{ y: 0x90 });
}

#[test]
fn test_sta() {
    test_op!("sta", ZeroPage,  [0x02]{a: 0x66} => [0x02, 0x66]{});
    test_op!("sta", ZeroPageX, [0x02]{a: 0x66, x:1} => [0x02, 0, 0x66]{});
    test_op!("sta", Absolute,  [0x04, 0]{a:0x66} => [0x04, 0, 0, 0x66]{});
    test_op!("sta", AbsoluteX, [0x03, 0]{a:0x66, x:1} => [0x03, 0, 0, 0x66]{});
    test_op!("sta", AbsoluteY, [0x03, 0]{a:0x66, y:1} => [0x03, 0, 0, 0x66]{});
    test_op!("sta", IndirectX, [0x02, 0, 0x05, 0, 0]{a: 0x66, x:1} => [0x02, 0, 0x05, 0, 0x66]{});
    test_op!("sta", IndirectY, [0x02, 0x04, 0, 0, 0]{a: 0x66, y:1} => [0x02, 0x04, 0, 0, 0x66]{});
}

#[test]
fn test_stx() {
    test_op!("stx", ZeroPage,  [0x02]{x: 0x66} => [0x02, 0x66]{});
    test_op!("stx", ZeroPageY, [0x02]{x: 0x66, y:1} => [0x02, 0, 0x66]{});
    test_op!("stx", Absolute,  [0x04, 0]{x: 0x66} => [0x04, 0, 0, 0x66]{});
}

#[test]
fn test_sty() {
    test_op!("sty", ZeroPage,  [0x02]{y: 0x66} => [0x02, 0x66]{});
    test_op!("sty", ZeroPageX, [0x02]{y: 0x66, x:1} => [0x02, 0, 0x66]{});
    test_op!("sty", Absolute,  [0x04, 0]{y: 0x66} => [0x04, 0, 0, 0x66]{});
}

#[test]
fn test_adc() {
    test_op!("adc", Immediate, [3]{a:2, p:1} => []{ a: 6 });
    test_op!("adc", Immediate, [255]{a:1, p:0} => []{ a: 0, p: 0b00000011 });
    test_op!("adc", Immediate, [127]{a:1, p:0} => []{ a: 128, p: 0b11000000 });
    test_op!("adc", Immediate, [200]{a:100} => []{ a: 44 });
    test_op!("adc", ZeroPage,  [0x02, 0x90]{a: 1} => []{ a: 0x91 });
    test_op!("adc", ZeroPageX, [0x02, 0, 0x90]{x:1, a: 1} => []{ a: 0x91 });
    test_op!("adc", Absolute,  [0x04, 0, 0, 0x90]{a:1} => []{ a: 0x91 });
    test_op!("adc", AbsoluteX, [0x03, 0, 0, 0x90]{x:1, a: 1} => []{ a: 0x91 });
    test_op!("adc", AbsoluteY, [0x03, 0, 0, 0x90]{y:1, a: 1} => []{ a: 0x91 });
    test_op!("adc", IndirectX, [0x02, 0, 0x05, 0, 0x90]{x:1, a: 1} => []{ a: 0x91 });
    test_op!("adc", IndirectY, [0x02, 0x04, 0, 0, 0x90]{y:1, a: 1} => []{ a: 0x91 });
}

#[test]
fn test_sbc() {
    test_op!("sbc", Immediate, [2]{a:10, p:1} => []{ a: 8 });
    test_op!("sbc", Immediate, [2]{a:10, p:0} => []{ a: 7 });
    test_op!("sbc", Immediate, [176]{a:80, p:1} => []{ a: 160, p: 0b11000000 });
    test_op!("sbc", ZeroPage,  [0x02, 0x90]{a: 0xFF, p: 1} => []{ a: 0x6f });
    test_op!("sbc", ZeroPageX, [0x02, 0, 0x90]{x:1, a: 0xFF, p: 1} => []{ a: 0x6f });
    test_op!("sbc", Absolute,  [0x04, 0, 0, 0x90]{a:0xFF, p: 1} => []{ a: 0x6f });
    test_op!("sbc", AbsoluteX, [0x03, 0, 0, 0x90]{x:1, a: 0xFF, p: 1} => []{ a: 0x6f });
    test_op!("sbc", AbsoluteY, [0x03, 0, 0, 0x90]{y:1, a: 0xFF, p: 1} => []{ a: 0x6f });
    test_op!("sbc", IndirectX, [0x02, 0, 0x05, 0, 0x90]{x:1, a: 0xFF, p: 1} => []{ a: 0x6f });
    test_op!("sbc", IndirectY, [0x02, 0x04, 0, 0, 0x90]{y:1, a: 0xFF, p: 1} => []{ a: 0x6f });
}

#[test]
fn test_cmp() {
    test_op!("cmp", Immediate, [10]{a:10} => []{ p: 0b00000011 });
    test_op!("cmp", Immediate, [100]{a:10} => []{ p: 0b10000000 });
    test_op!("cmp", Immediate, [10]{a:100} => []{ p: 0b00000001 });
    test_op!("cmp", ZeroPage,  [0x02, 10]{a: 10} => []{ p: 0b00000011 });
    test_op!("cmp", ZeroPageX, [0x02, 0, 10]{x:1, a: 10} => []{ p: 0b00000011 });
    test_op!("cmp", Absolute,  [0x04, 0, 0, 10]{a:10} => []{ p: 0b00000011  });
    test_op!("cmp", AbsoluteX, [0x03, 0, 0, 10]{x:1, a: 10} => []{ p: 0b00000011 });
    test_op!("cmp", AbsoluteY, [0x03, 0, 0, 10]{y:1, a: 10} => []{ p: 0b00000011 });
    test_op!("cmp", IndirectX, [0x02, 0, 0x05, 0, 10]{x:1, a: 10} => []{ p: 0b00000011 });
    test_op!("cmp", IndirectY, [0x02, 0x04, 0, 0, 10]{y:1, a: 10} => []{ p: 0b00000011 });
}

#[test]
fn test_cpx() {
    test_op!("cpx", Immediate, [10]{x:10} => []{ p: 0b00000011 });
    test_op!("cpx", Immediate, [100]{x:10} => []{ p: 0b10000000 });
    test_op!("cpx", Immediate, [10]{x:100} => []{ p: 0b00000001 });
    test_op!("cpx", ZeroPage,  [0x02, 10]{x: 10} => []{ p: 0b00000011 });
    test_op!("cpx", Absolute,  [0x04, 0, 0, 10]{x:10} => []{ p: 0b00000011  });
}

#[test]
fn test_cpy() {
    test_op!("cpy", Immediate, [10]{y:10} => []{ p: 0b00000011 });
    test_op!("cpy", Immediate, [100]{y:10} => []{ p: 0b10000000 });
    test_op!("cpy", Immediate, [10]{y:100} => []{ p: 0b00000001 });
    test_op!("cpy", ZeroPage,  [0x02, 10]{y: 10} => []{ p: 0b00000011 });
    test_op!("cpy", Absolute,  [0x04, 0, 0, 10]{y:10} => []{ p: 0b00000011  });
}

#[test]
fn test_and() {
    test_op!("and", Immediate, [0b00001111]{a:0b01010101} => []{ a: 0b00000101, p: 0 });
    test_op!("and", Immediate, [0b10001111]{a:0b11010101} => []{ a: 0b10000101, p: 0b10000000 });
    test_op!("and", Immediate, [0]{a:0b11010101} => []{ a: 0, p: 0b00000010 });
    test_op!("and", ZeroPage,  [0x02, 0xFF]{a: 0xF0} => []{a: 0xF0});
    test_op!("and", ZeroPageX, [0x02, 0, 0xFF]{x:1, a: 0xF0} => []{a: 0xF0});
    test_op!("and", Absolute,  [0x04, 0, 0, 0xFF]{a:0xF0} => []{a: 0xF0});
    test_op!("and", AbsoluteX, [0x03, 0, 0, 0xFF]{x:1, a: 0xF0} => []{a: 0xF0});
    test_op!("and", AbsoluteY, [0x03, 0, 0, 0xFF]{y:1, a: 0xF0} => []{a: 0xF0});
    test_op!("and", IndirectX, [0x02, 0, 0x05, 0, 0xFF]{x:1, a: 0xF0} => []{a: 0xF0});
    test_op!("and", IndirectY, [0x02, 0x04, 0, 0, 0xFF]{y:1, a: 0xF0} => []{a: 0xF0});
}

#[test]
fn test_ora() {
    test_op!("ora", Immediate, [0b00001111]{a:0b01010101} => []{ a: 0b01011111, p: 0 });
    test_op!("ora", Immediate, [0b10001111]{a:0b01010101} => []{ a: 0b11011111, p: 0b10000000 });
    test_op!("ora", Immediate, [0]{a:0} => []{ a: 0, p: 0b00000010 });
    test_op!("ora", ZeroPage,  [0x02, 0xFF]{a: 0xF0} => []{a: 0xFF});
    test_op!("ora", ZeroPageX, [0x02, 0, 0xFF]{x:1, a: 0xF0} => []{a: 0xFF});
    test_op!("ora", Absolute,  [0x04, 0, 0, 0xFF]{a:0xF0} => []{a: 0xFF});
    test_op!("ora", AbsoluteX, [0x03, 0, 0, 0xFF]{x:1, a: 0xF0} => []{a: 0xFF});
    test_op!("ora", AbsoluteY, [0x03, 0, 0, 0xFF]{y:1, a: 0xF0} => []{a: 0xFF});
    test_op!("ora", IndirectX, [0x02, 0, 0x05, 0, 0xFF]{x:1, a: 0xF0} => []{a: 0xFF});
    test_op!("ora", IndirectY, [0x02, 0x04, 0, 0, 0xFF]{y:1, a: 0xF0} => []{a: 0xFF});
}

#[test]
fn test_eor() {
    test_op!("eor", Immediate, [0b00001111]{a:0b01010101} => []{ a: 0b01011010, p: 0 });
    test_op!("eor", Immediate, [0b10001111]{a:0b01010101} => []{ a: 0b11011010, p: 0b10000000 });
    test_op!("eor", Immediate, [0xFF]{a:0xFF} => []{ a: 0, p: 0b00000010 });
    test_op!("eor", ZeroPage,  [0x02, 0xFF]{a: 0xF0} => []{a: 0x0F});
    test_op!("eor", ZeroPageX, [0x02, 0, 0xFF]{x:1, a: 0xF0} => []{a: 0x0F});
    test_op!("eor", Absolute,  [0x04, 0, 0, 0xFF]{a:0xF0} => []{a: 0x0F});
    test_op!("eor", AbsoluteX, [0x03, 0, 0, 0xFF]{x:1, a: 0xF0} => []{a: 0x0F});
    test_op!("eor", AbsoluteY, [0x03, 0, 0, 0xFF]{y:1, a: 0xF0} => []{a: 0x0F});
    test_op!("eor", IndirectX, [0x02, 0, 0x05, 0, 0xFF]{x:1, a: 0xF0} => []{a: 0x0F});
    test_op!("eor", IndirectY, [0x02, 0x04, 0, 0, 0xFF]{y:1, a: 0xF0} => []{a: 0x0F});
}

#[test]
fn test_bit() {
    test_op!("bit", ZeroPage,  [0x02, 0x00]{a: 0x0F} => []{p: 0b00000010});
    test_op!("bit", ZeroPage,  [0x02, 0xF0]{a: 0xFF} => []{p: 0b11000000});
    test_op!("bit", Absolute,  [0x03, 0, 0xF0]{a: 0xFF} => []{p: 0b11000000});
}

#[test]
fn test_rol() {
    test_op!("rol", ZeroPage,  [0x02, 0xFF]{p:1} => [0x02, 0xFF]{p: 0b10000001});
    test_op!("rol", ZeroPage,  [0x02, 0xFF]{p:0} => [0x02, 0xFE]{p: 0b10000001});
    test_op!("rol", ZeroPage,  [0x02, 0b10000000]{p:0} => [0x02, 0]{p: 0b00000011});
    test_op!("rol", ZeroPageX, [0x02, 0, 0xFF]{p:1, x: 1} => [0x02, 0, 0xFF]{p: 0b10000001});
    test_op!("rol", Absolute,  [0x03, 0, 0xFF]{p:1} => [0x03, 0, 0xFF]{p: 0b10000001});
    test_op!("rol", AbsoluteX, [0x03, 0, 0, 0xFF]{p:1, x: 1} => [0x03, 0, 0, 0xFF]{p: 0b10000001});
}

#[test]
fn test_ror() {
    test_op!("ror", ZeroPage,  [0x02, 0xFF]{p:1} => [0x02, 0xFF]{p: 0b10000001});
    test_op!("ror", ZeroPage,  [0x02, 0xFF]{p:0} => [0x02, 0x7f]{p: 0b00000001});
    test_op!("ror", ZeroPage,  [0x02, 1]{p:0} => [0x02, 0]{p: 0b00000011});
    test_op!("ror", ZeroPageX,  [0x02, 0, 1]{p:0, x: 1} => [0x02, 0]{p: 0b00000011});
    test_op!("ror", Absolute,  [0x03, 0, 1]{p:0} => [0x03, 0]{p: 0b00000011});
    test_op!("ror", AbsoluteX,  [0x02, 0, 1]{p:0, x: 1} => [0x02, 0]{p: 0b00000011});
    test_op!("ror", NoMode, []{a: 2} => []{a: 1});
}

#[test]
fn test_asl() {
    test_op!("asl", ZeroPage,  [0x02, 0xFF]{p:1} => [0x02, 0xFE]{p: 0b10000001});
    test_op!("asl", ZeroPage,  [0x02, 0xFF]{p:0} => [0x02, 0xFE]{p: 0b10000001});
    test_op!("asl", ZeroPage,  [0x02, 0b10000000]{} => [0x02, 0]{p: 0b00000011});
    test_op!("asl", ZeroPageX, [0x02, 0, 1]{x: 1} => [0x02, 0, 2]{});
    test_op!("asl", Absolute,  [0x03, 0, 1]{} => [0x03, 0, 2]{});
    test_op!("asl", AbsoluteX, [0x03, 0, 0, 1]{x: 1} => [0x03, 0, 0, 2]{});
    test_op!("asl", NoMode, []{a: 1} => []{a: 2});
}

#[test]
fn test_lsr() {
    test_op!("lsr", ZeroPage,  [0x02, 1]{p:1} => [0x02, 0]{p: 0b00000011});
    test_op!("lsr", ZeroPage,  [0x02, 1]{p:0} => [0x02, 0]{p: 0b00000011});
    test_op!("lsr", ZeroPageX, [0x02, 0, 2]{x: 1} => [0x02, 0, 1]{});
    test_op!("lsr", Absolute,  [0x03, 0, 2]{} => [0x03, 0, 1]{});
    test_op!("lsr", AbsoluteX, [0x03, 0, 0, 2]{x: 1} => [0x03, 0, 0, 1]{});
    test_op!("lsr", NoMode, []{a: 2} => []{a: 1});
}

#[test]
fn test_inc() {
    test_op!("inc", ZeroPage,  [0x02, 255]{} => [0x02, 0]{p: 0b00000010});
    test_op!("inc", ZeroPage,  [0x02, 127]{} => [0x02, 128]{p: 0b10000000});
    test_op!("inc", ZeroPageX, [0x02, 0, 2]{x: 1} => [0x02, 0, 3]{});
    test_op!("inc", Absolute,  [0x03, 0, 2]{} => [0x03, 0, 3]{});
    test_op!("inc", AbsoluteX, [0x03, 0, 0, 2]{x: 1} => [0x03, 0, 0, 3]{});
}

#[test]
fn test_dec() {
    test_op!("dec", ZeroPage,  [0x02, 0]{} => [0x02, 255]{p: 0b10000000});
    test_op!("dec", ZeroPage,  [0x02, 1]{} => [0x02, 0]{p: 0b00000010});
    test_op!("dec", ZeroPageX, [0x02, 0, 2]{x: 1} => [0x02, 0, 1]{});
    test_op!("dec", Absolute,  [0x03, 0, 2]{} => [0x03, 0, 1]{});
    test_op!("dec", AbsoluteX, [0x03, 0, 0, 2]{x: 1} => [0x03, 0, 0, 1]{});
}

#[test]
fn test_inx() {
    test_op!("inx", NoMode,  []{x: 255} => []{x: 0, p: 0b00000010});
    test_op!("inx", NoMode,  []{x: 127} => []{x: 128, p: 0b10000000});
}

#[test]
fn test_dex() {
    test_op!("dex", NoMode,  []{x: 1} => []{x: 0, p: 0b00000010});
    test_op!("dex", NoMode,  []{x: 0} => []{x: 255, p: 0b10000000});
}

#[test]
fn test_iny() {
    test_op!("iny", NoMode,  []{y: 255} => []{y: 0, p: 0b00000010});
    test_op!("iny", NoMode,  []{y: 127} => []{y: 128, p: 0b10000000});
}

#[test]
fn test_dey() {
    test_op!("dey", NoMode,  []{y: 1} => []{y: 0, p: 0b00000010});
    test_op!("dey", NoMode,  []{y: 0} => []{y: 255, p: 0b10000000});
}

#[test]
fn test_tax() {
    test_op!("tax", NoMode,  []{a: 1} => []{a: 1, x: 1, p: 0b00000000});
    test_op!("tax", NoMode,  []{a: 0} => []{a: 0, x: 0, p: 0b00000010});
    test_op!("tax", NoMode,  []{a: 128} => []{a: 128, x: 128, p: 0b10000000});
}

#[test]
fn test_tay() {
    test_op!("tay", NoMode,  []{a: 1} => []{a: 1, y: 1, p: 0b00000000});
    test_op!("tay", NoMode,  []{a: 0} => []{a: 0, y: 0, p: 0b00000010});
    test_op!("tay", NoMode,  []{a: 128} => []{a: 128, y: 128, p: 0b10000000});
}
#[test]
fn test_txa() {
    test_op!("txa", NoMode,  []{x: 1} => []{a: 1, x: 1, p: 0b00000000});
    test_op!("txa", NoMode,  []{x: 0} => []{a: 0, x: 0, p: 0b00000010});
    test_op!("txa", NoMode,  []{x: 128} => []{a: 128, x: 128, p: 0b10000000});
}

#[test]
fn test_tya() {
    test_op!("tya", NoMode,  []{y: 1} => []{a: 1, y: 1, p: 0b00000000});
    test_op!("tya", NoMode,  []{y: 0} => []{a: 0, y: 0, p: 0b00000010});
    test_op!("tya", NoMode,  []{y: 128} => []{a: 128, y: 128, p: 0b10000000});
}

#[test]
fn test_tsx() {
    test_op!("tsx", NoMode,  []{sp: 1} => []{sp: 1, x: 1, p: 0b00000000});
    test_op!("tsx", NoMode,  []{sp: 0} => []{sp: 0, x: 0, p: 0b00000010});
    test_op!("tsx", NoMode,  []{sp: 128} => []{sp: 128, x: 128, p: 0b10000000});
}

#[test]
fn test_txs() {
    test_op!("txs", NoMode,  []{x: 1} => []{sp: 1, x: 1, p: 0});
}

#[test]
fn test_flag_ops() {
    test_op!("clc", NoMode, []{p: 0b11111111} => []{p: 0b11111110});
    test_op!("sec", NoMode, []{p: 0} => []{p: 1});
    test_op!("cli", NoMode, []{p: 0b11111111} => []{p: 0b11111011});
    test_op!("sei", NoMode, []{p: 0} => []{p: 0b00000100});
    test_op!("clv", NoMode, []{p: 0b11111111} => []{p: 0b10111111});
    test_op!("cld", NoMode, []{p: 0b11111111} => []{p: 0b11110111});
    test_op!("sed", NoMode, []{p: 0} => []{p: 0b00001000});
}

#[test]
fn test_bpl() {
    let cpu = test_op!("bpl", NoMode, [10]{p: 0b10000000} => []{pc: 2});
    assert_eq!(cpu.bus.cycles, 2);

    let cpu = test_op!("bpl", NoMode, [10]{p: 0} => []{pc: 12});
    assert_eq!(cpu.bus.cycles, 3);

    // Test page boundary cross
    let mut cpu = build_cpu!([0]);
    cpu.pc = 0x00FE;
    cpu.bus.ram[0x00FE] = 1;
    cpu.bpl();
    assert!(cross(0x00FF, 1));
    assert_eq!(cpu.pc, 0x0100);
    assert_eq!(cpu.bus.cycles, 3); // Because we call bpl directly, it's only 3 cycles
}

#[test]
fn test_bmi() {
    let cpu = test_op!("bmi", NoMode, [10]{p: 0} => []{pc: 2});
    assert_eq!(cpu.bus.cycles, 2);

    let cpu = test_op!("bmi", NoMode, [10]{p: 0b10000000} => []{pc: 12});
    assert_eq!(cpu.bus.cycles, 3);

    // Test page boundary cross
    let mut cpu = build_cpu!([0]);
    cpu.pc = 0x00FE;
    cpu.bus.ram[0x00FE] = 1;
    cpu.p = 0b10000000;
    cpu.bmi();
    assert!(cross(0x00FF, 1));
    assert_eq!(cpu.pc, 0x0100);
    assert_eq!(cpu.bus.cycles, 3); // Because we call bmi directly, it's only 3 cycles
}

#[test]
fn test_bvc() {
    let cpu = test_op!("bvc", NoMode, [10]{p: 0b01000000} => []{pc: 2});
    assert_eq!(cpu.bus.cycles, 2);

    let cpu = test_op!("bvc", NoMode, [10]{p: 0} => []{pc: 12});
    assert_eq!(cpu.bus.cycles, 3);
}

#[test]
fn test_bvs() {
    let cpu = test_op!("bvs", NoMode, [10]{p: 0b00000000} => []{pc: 2});
    assert_eq!(cpu.bus.cycles, 2);

    let cpu = test_op!("bvs", NoMode, [10]{p: 0b01000000} => []{pc: 12});
    assert_eq!(cpu.bus.cycles, 3);
}

#[test]
fn test_bcc() {
    let cpu = test_op!("bcc", NoMode, [10]{p: 0b00000001} => []{pc: 2});
    assert_eq!(cpu.bus.cycles, 2);

    let cpu = test_op!("bcc", NoMode, [10]{p: 0} => []{pc: 12});
    assert_eq!(cpu.bus.cycles, 3);
}

#[test]
fn test_bcs() {
    let cpu = test_op!("bcs", NoMode, [10]{p: 0b00000000} => []{pc: 2});
    assert_eq!(cpu.bus.cycles, 2);

    let cpu = test_op!("bcs", NoMode, [10]{p: 0b00000001} => []{pc: 12});
    assert_eq!(cpu.bus.cycles, 3);
}

#[test]
fn test_bne() {
    let cpu = test_op!("bne", NoMode, [10]{p: 0b00000010} => []{pc: 2});
    assert_eq!(cpu.bus.cycles, 2);

    let cpu = test_op!("bne", NoMode, [10]{p: 0} => []{pc: 12});
    assert_eq!(cpu.bus.cycles, 3);
}

#[test]
fn test_beq() {
    let cpu = test_op!("beq", NoMode, [10]{p: 0b00000000} => []{pc: 2});
    assert_eq!(cpu.bus.cycles, 2);

    let cpu = test_op!("beq", NoMode, [10]{p: 0b00000010} => []{pc: 12});
    assert_eq!(cpu.bus.cycles, 3);
}

#[test]
fn test_jmp() {
    test_op!("jmp", Absolute, [10, 0]{} => []{pc: 10});
    test_op!("jmp", Indirect, [3, 0, 10, 0]{} => []{pc: 10});

    // Test page boundary bug
    let mut cpu = build_cpu!([0]);
    cpu.pc = 0;
    cpu.bus.ram[0] = 0xFF;
    cpu.bus.ram[1] = 0x01;
    cpu.bus.ram[0x01FF] = 0x11;
    cpu.bus.ram[0x0100] = 0x22;
    cpu.jmp(Indirect);
    assert_eq!(cpu.pc, 0x2211);
}

#[test]
fn test_jsr_ret() {
    let mut cpu = build_cpu!([10, 0]);
    cpu.reset();
    cpu.pc = 0;
    cpu.bus.cycles = 0;
    assert_eq!(cpu.sp, 0xFF);
    cpu.jsr();
    // Actual cycles of JSR is 6, but we skip one cycle by
    // calling jsr() directly instead of reading the opcode
    assert_eq!(cpu.bus.cycles, 5);
    assert_eq!(cpu.pc, 10);
    assert_eq!(cpu.sp, 0xFF - 2);
    assert_eq!(cpu.bus.ram[cpu.sp as usize + 0x101], 1);
    assert_eq!(cpu.bus.ram[cpu.sp as usize + 0x102], 0);
    cpu.rts();
    assert_eq!(cpu.pc, 2);
    assert_eq!(cpu.sp, 0xFF);
}

#[test]
fn test_brk() {
    let mut cpu = build_cpu!([0]);
    cpu.reset();
    cpu.pc = 0x0201;
    cpu.p = 179;
    cpu.bus.cycles = 0;
    cpu.brk();
    // Actual cycles of BRK is 7, but we skip one cycle by
    // calling brk() directly instead of reading the opcode
    assert_eq!(cpu.bus.cycles, 6);
    assert_eq!(cpu.pc, 0);

    cpu.bus.cycles = 0;
    cpu.rti();
    assert_eq!(cpu.p, 179);
}

#[test]
fn test_pha_pla() {
    let mut cpu = test_op!("pha", NoMode, []{a: 0x57} => []{});
    cpu.a = 0;
    cpu.bus.cycles = 0;
    cpu.pla();
    assert_eq!(cpu.a, 0x57);
    assert_eq!(cpu.bus.cycles, 3); // Really 4 once you add an opcode read.
}

#[test]
fn test_php_plp() {
    let mut cpu = test_op!("php", NoMode, []{p: 0x57} => []{});
    cpu.p = 0;
    cpu.bus.cycles = 0;
    cpu.plp();
    assert_eq!(cpu.p, 0x57 & 0b1100_1111);
    assert_eq!(cpu.bus.cycles, 3); // Really 4 once you add an opcode read.
}

#[derive(Debug)]
struct Op {
    code: u8,
    size: u16,
    cycles: u64,
    check: bool,
    mask: u8,
}

fn opcode(name: &str, mode: Mode) -> Op {
    match (name, mode) {
        ("adc", Immediate) => Op {
            code: 0x69,
            size: 2,
            cycles: 2,
            check: false,
            mask: 0b11000011,
        },
        ("adc", ZeroPage) => Op {
            code: 0x65,
            size: 2,
            cycles: 3,
            check: false,
            mask: 0b11000011,
        },
        ("adc", ZeroPageX) => Op {
            code: 0x75,
            size: 2,
            cycles: 4,
            check: false,
            mask: 0b11000011,
        },
        ("adc", Absolute) => Op {
            code: 0x6D,
            size: 3,
            cycles: 4,
            check: false,
            mask: 0b11000011,
        },
        ("adc", AbsoluteX) => Op {
            code: 0x7D,
            size: 3,
            cycles: 4,
            check: true,
            mask: 0b11000011,
        },
        ("adc", AbsoluteY) => Op {
            code: 0x79,
            size: 3,
            cycles: 4,
            check: true,
            mask: 0b11000011,
        },
        ("adc", IndirectX) => Op {
            code: 0x61,
            size: 2,
            cycles: 6,
            check: false,
            mask: 0b11000011,
        },
        ("adc", IndirectY) => Op {
            code: 0x71,
            size: 2,
            cycles: 5,
            check: true,
            mask: 0b11000011,
        },
        ("and", Immediate) => Op {
            code: 0x29,
            size: 2,
            cycles: 2,
            check: false,
            mask: 0b10000010,
        },
        ("and", ZeroPage) => Op {
            code: 0x25,
            size: 2,
            cycles: 3,
            check: false,
            mask: 0b10000010,
        },
        ("and", ZeroPageX) => Op {
            code: 0x35,
            size: 2,
            cycles: 4,
            check: false,
            mask: 0b10000010,
        },
        ("and", Absolute) => Op {
            code: 0x2D,
            size: 3,
            cycles: 4,
            check: false,
            mask: 0b10000010,
        },
        ("and", AbsoluteX) => Op {
            code: 0x3D,
            size: 3,
            cycles: 4,
            check: true,
            mask: 0b10000010,
        },
        ("and", AbsoluteY) => Op {
            code: 0x39,
            size: 3,
            cycles: 4,
            check: true,
            mask: 0b10000010,
        },
        ("and", IndirectX) => Op {
            code: 0x21,
            size: 2,
            cycles: 6,
            check: false,
            mask: 0b10000010,
        },
        ("and", IndirectY) => Op {
            code: 0x31,
            size: 2,
            cycles: 5,
            check: true,
            mask: 0b10000010,
        },
        ("asl", NoMode) => Op {
            code: 0x0A,
            size: 1,
            cycles: 2,
            check: false,
            mask: 0b10000011,
        },
        ("asl", ZeroPage) => Op {
            code: 0x06,
            size: 2,
            cycles: 5,
            check: false,
            mask: 0b10000011,
        },
        ("asl", ZeroPageX) => Op {
            code: 0x16,
            size: 2,
            cycles: 6,
            check: false,
            mask: 0b10000011,
        },
        ("asl", Absolute) => Op {
            code: 0x0E,
            size: 3,
            cycles: 6,
            check: false,
            mask: 0b10000011,
        },
        ("asl", AbsoluteX) => Op {
            code: 0x1E,
            size: 3,
            cycles: 7,
            check: false,
            mask: 0b10000011,
        },
        ("bcc", NoMode) => Op {
            code: 0x90,
            size: 0,
            cycles: 0,
            check: true,
            mask: 0b00000000,
        },
        ("bcs", NoMode) => Op {
            code: 0xB0,
            size: 0,
            cycles: 0,
            check: true,
            mask: 0b00000000,
        },
        ("beq", NoMode) => Op {
            code: 0xF0,
            size: 0,
            cycles: 0,
            check: true,
            mask: 0b00000000,
        },
        ("bit", ZeroPage) => Op {
            code: 0x24,
            size: 2,
            cycles: 3,
            check: false,
            mask: 0b11000010,
        },
        ("bit", Absolute) => Op {
            code: 0x2C,
            size: 3,
            cycles: 4,
            check: false,
            mask: 0b11000010,
        },
        ("bmi", NoMode) => Op {
            code: 0x30,
            size: 0,
            cycles: 0,
            check: true,
            mask: 0b00000000,
        },
        ("bne", NoMode) => Op {
            code: 0xD0,
            size: 0,
            cycles: 0,
            check: true,
            mask: 0b00000000,
        },
        ("bpl", NoMode) => Op {
            code: 0x10,
            size: 0,
            cycles: 0,
            check: true,
            mask: 0b00000000,
        },
        ("brk", NoMode) => Op {
            code: 0x00,
            size: 0,
            cycles: 7,
            check: false,
            mask: 0b00010000,
        },
        ("bvc", NoMode) => Op {
            code: 0x50,
            size: 0,
            cycles: 0,
            check: true,
            mask: 0b00000000,
        },
        ("bvs", NoMode) => Op {
            code: 0x70,
            size: 0,
            cycles: 0,
            check: true,
            mask: 0b00000000,
        },
        ("clc", NoMode) => Op {
            code: 0x18,
            size: 1,
            cycles: 2,
            check: false,
            mask: 0b00000001,
        },
        ("cld", NoMode) => Op {
            code: 0xD8,
            size: 1,
            cycles: 2,
            check: false,
            mask: 0b00001000,
        },
        ("cli", NoMode) => Op {
            code: 0x58,
            size: 1,
            cycles: 2,
            check: false,
            mask: 0b00000100,
        },
        ("clv", NoMode) => Op {
            code: 0xB8,
            size: 1,
            cycles: 2,
            check: false,
            mask: 0b01000000,
        },
        ("cmp", Immediate) => Op {
            code: 0xC9,
            size: 2,
            cycles: 2,
            check: false,
            mask: 0b10000011,
        },
        ("cmp", ZeroPage) => Op {
            code: 0xC5,
            size: 2,
            cycles: 3,
            check: false,
            mask: 0b10000011,
        },
        ("cmp", ZeroPageX) => Op {
            code: 0xD5,
            size: 2,
            cycles: 4,
            check: false,
            mask: 0b10000011,
        },
        ("cmp", Absolute) => Op {
            code: 0xCD,
            size: 3,
            cycles: 4,
            check: false,
            mask: 0b10000011,
        },
        ("cmp", AbsoluteX) => Op {
            code: 0xDD,
            size: 3,
            cycles: 4,
            check: true,
            mask: 0b10000011,
        },
        ("cmp", AbsoluteY) => Op {
            code: 0xD9,
            size: 3,
            cycles: 4,
            check: true,
            mask: 0b10000011,
        },
        ("cmp", IndirectX) => Op {
            code: 0xC1,
            size: 2,
            cycles: 6,
            check: false,
            mask: 0b10000011,
        },
        ("cmp", IndirectY) => Op {
            code: 0xD1,
            size: 2,
            cycles: 5,
            check: true,
            mask: 0b10000011,
        },
        ("cpx", Immediate) => Op {
            code: 0xE0,
            size: 2,
            cycles: 2,
            check: false,
            mask: 0b10000011,
        },
        ("cpx", ZeroPage) => Op {
            code: 0xE4,
            size: 2,
            cycles: 3,
            check: false,
            mask: 0b10000011,
        },
        ("cpx", Absolute) => Op {
            code: 0xEC,
            size: 3,
            cycles: 4,
            check: false,
            mask: 0b10000011,
        },
        ("cpy", Immediate) => Op {
            code: 0xC0,
            size: 2,
            cycles: 2,
            check: false,
            mask: 0b10000011,
        },
        ("cpy", ZeroPage) => Op {
            code: 0xC4,
            size: 2,
            cycles: 3,
            check: false,
            mask: 0b10000011,
        },
        ("cpy", Absolute) => Op {
            code: 0xCC,
            size: 3,
            cycles: 4,
            check: false,
            mask: 0b10000011,
        },
        ("dec", ZeroPage) => Op {
            code: 0xC6,
            size: 2,
            cycles: 5,
            check: false,
            mask: 0b10000010,
        },
        ("dec", ZeroPageX) => Op {
            code: 0xD6,
            size: 2,
            cycles: 6,
            check: false,
            mask: 0b10000010,
        },
        ("dec", Absolute) => Op {
            code: 0xCE,
            size: 3,
            cycles: 6,
            check: false,
            mask: 0b10000010,
        },
        ("dec", AbsoluteX) => Op {
            code: 0xDE,
            size: 3,
            cycles: 7,
            check: false,
            mask: 0b10000010,
        },
        ("dex", NoMode) => Op {
            code: 0xCA,
            size: 1,
            cycles: 2,
            check: false,
            mask: 0b10000010,
        },
        ("dey", NoMode) => Op {
            code: 0x88,
            size: 1,
            cycles: 2,
            check: false,
            mask: 0b10000010,
        },
        ("eor", Immediate) => Op {
            code: 0x49,
            size: 2,
            cycles: 2,
            check: false,
            mask: 0b10000010,
        },
        ("eor", ZeroPage) => Op {
            code: 0x45,
            size: 2,
            cycles: 3,
            check: false,
            mask: 0b10000010,
        },
        ("eor", ZeroPageX) => Op {
            code: 0x55,
            size: 2,
            cycles: 4,
            check: false,
            mask: 0b10000010,
        },
        ("eor", Absolute) => Op {
            code: 0x4D,
            size: 3,
            cycles: 4,
            check: false,
            mask: 0b10000010,
        },
        ("eor", AbsoluteX) => Op {
            code: 0x5D,
            size: 3,
            cycles: 4,
            check: true,
            mask: 0b10000010,
        },
        ("eor", AbsoluteY) => Op {
            code: 0x59,
            size: 3,
            cycles: 4,
            check: true,
            mask: 0b10000010,
        },
        ("eor", IndirectX) => Op {
            code: 0x41,
            size: 2,
            cycles: 6,
            check: false,
            mask: 0b10000010,
        },
        ("eor", IndirectY) => Op {
            code: 0x51,
            size: 2,
            cycles: 5,
            check: true,
            mask: 0b10000010,
        },
        ("inc", ZeroPage) => Op {
            code: 0xE6,
            size: 2,
            cycles: 5,
            check: false,
            mask: 0b10000010,
        },
        ("inc", ZeroPageX) => Op {
            code: 0xF6,
            size: 2,
            cycles: 6,
            check: false,
            mask: 0b10000010,
        },
        ("inc", Absolute) => Op {
            code: 0xEE,
            size: 3,
            cycles: 6,
            check: false,
            mask: 0b10000010,
        },
        ("inc", AbsoluteX) => Op {
            code: 0xFE,
            size: 3,
            cycles: 7,
            check: false,
            mask: 0b10000010,
        },
        ("inx", NoMode) => Op {
            code: 0xE8,
            size: 1,
            cycles: 2,
            check: false,
            mask: 0b10000010,
        },
        ("iny", NoMode) => Op {
            code: 0xC8,
            size: 1,
            cycles: 2,
            check: false,
            mask: 0b10000010,
        },
        ("jmp", Absolute) => Op {
            code: 0x4C,
            size: 0,
            cycles: 3,
            check: false,
            mask: 0b00000000,
        },
        ("jmp", Indirect) => Op {
            code: 0x6C,
            size: 0,
            cycles: 5,
            check: false,
            mask: 0b00000000,
        },
        ("jsr", Absolute) => Op {
            code: 0x20,
            size: 0,
            cycles: 6,
            check: false,
            mask: 0b00000000,
        },
        ("lda", Immediate) => Op {
            code: 0xA9,
            size: 2,
            cycles: 2,
            check: false,
            mask: 0b10000010,
        },
        ("lda", ZeroPage) => Op {
            code: 0xA5,
            size: 2,
            cycles: 3,
            check: false,
            mask: 0b10000010,
        },
        ("lda", ZeroPageX) => Op {
            code: 0xB5,
            size: 2,
            cycles: 4,
            check: false,
            mask: 0b10000010,
        },
        ("lda", Absolute) => Op {
            code: 0xAD,
            size: 3,
            cycles: 4,
            check: false,
            mask: 0b10000010,
        },
        ("lda", AbsoluteX) => Op {
            code: 0xBD,
            size: 3,
            cycles: 4,
            check: true,
            mask: 0b10000010,
        },
        ("lda", AbsoluteY) => Op {
            code: 0xB9,
            size: 3,
            cycles: 4,
            check: true,
            mask: 0b10000010,
        },
        ("lda", IndirectX) => Op {
            code: 0xA1,
            size: 2,
            cycles: 6,
            check: false,
            mask: 0b10000010,
        },
        ("lda", IndirectY) => Op {
            code: 0xB1,
            size: 2,
            cycles: 5,
            check: true,
            mask: 0b10000010,
        },
        ("ldx", Immediate) => Op {
            code: 0xA2,
            size: 2,
            cycles: 2,
            check: false,
            mask: 0b10000010,
        },
        ("ldx", ZeroPage) => Op {
            code: 0xA6,
            size: 2,
            cycles: 3,
            check: false,
            mask: 0b10000010,
        },
        ("ldx", ZeroPageY) => Op {
            code: 0xB6,
            size: 2,
            cycles: 4,
            check: false,
            mask: 0b10000010,
        },
        ("ldx", Absolute) => Op {
            code: 0xAE,
            size: 3,
            cycles: 4,
            check: false,
            mask: 0b10000010,
        },
        ("ldx", AbsoluteY) => Op {
            code: 0xBE,
            size: 3,
            cycles: 4,
            check: true,
            mask: 0b10000010,
        },
        ("ldy", Immediate) => Op {
            code: 0xA0,
            size: 2,
            cycles: 2,
            check: false,
            mask: 0b10000010,
        },
        ("ldy", ZeroPage) => Op {
            code: 0xA4,
            size: 2,
            cycles: 3,
            check: false,
            mask: 0b10000010,
        },
        ("ldy", ZeroPageX) => Op {
            code: 0xB4,
            size: 2,
            cycles: 4,
            check: false,
            mask: 0b10000010,
        },
        ("ldy", Absolute) => Op {
            code: 0xAC,
            size: 3,
            cycles: 4,
            check: false,
            mask: 0b10000010,
        },
        ("ldy", AbsoluteX) => Op {
            code: 0xBC,
            size: 3,
            cycles: 4,
            check: true,
            mask: 0b10000010,
        },
        ("lsr", NoMode) => Op {
            code: 0x4A,
            size: 1,
            cycles: 2,
            check: false,
            mask: 0b10000011,
        },
        ("lsr", ZeroPage) => Op {
            code: 0x46,
            size: 2,
            cycles: 5,
            check: false,
            mask: 0b10000011,
        },
        ("lsr", ZeroPageX) => Op {
            code: 0x56,
            size: 2,
            cycles: 6,
            check: false,
            mask: 0b10000011,
        },
        ("lsr", Absolute) => Op {
            code: 0x4E,
            size: 3,
            cycles: 6,
            check: false,
            mask: 0b10000011,
        },
        ("lsr", AbsoluteX) => Op {
            code: 0x5E,
            size: 3,
            cycles: 7,
            check: false,
            mask: 0b10000011,
        },
        ("nop", NoMode) => Op {
            code: 0xEA,
            size: 1,
            cycles: 2,
            check: false,
            mask: 0b00000000,
        },
        ("ora", Immediate) => Op {
            code: 0x09,
            size: 2,
            cycles: 2,
            check: false,
            mask: 0b10000010,
        },
        ("ora", ZeroPage) => Op {
            code: 0x05,
            size: 2,
            cycles: 3,
            check: false,
            mask: 0b10000010,
        },
        ("ora", ZeroPageX) => Op {
            code: 0x15,
            size: 2,
            cycles: 4,
            check: false,
            mask: 0b10000010,
        },
        ("ora", Absolute) => Op {
            code: 0x0D,
            size: 3,
            cycles: 4,
            check: false,
            mask: 0b10000010,
        },
        ("ora", AbsoluteX) => Op {
            code: 0x1D,
            size: 3,
            cycles: 4,
            check: true,
            mask: 0b10000010,
        },
        ("ora", AbsoluteY) => Op {
            code: 0x19,
            size: 3,
            cycles: 4,
            check: true,
            mask: 0b10000010,
        },
        ("ora", IndirectX) => Op {
            code: 0x01,
            size: 2,
            cycles: 6,
            check: false,
            mask: 0b10000010,
        },
        ("ora", IndirectY) => Op {
            code: 0x11,
            size: 2,
            cycles: 5,
            check: true,
            mask: 0b10000010,
        },
        ("pha", NoMode) => Op {
            code: 0x48,
            size: 1,
            cycles: 3,
            check: false,
            mask: 0b00000000,
        },
        ("php", NoMode) => Op {
            code: 0x08,
            size: 1,
            cycles: 3,
            check: false,
            mask: 0b00000000,
        },
        ("pla", NoMode) => Op {
            code: 0x68,
            size: 1,
            cycles: 4,
            check: false,
            mask: 0b10000010,
        },
        ("plp", NoMode) => Op {
            code: 0x28,
            size: 1,
            cycles: 4,
            check: false,
            mask: 0b11011111,
        },
        ("rol", NoMode) => Op {
            code: 0x2A,
            size: 1,
            cycles: 2,
            check: false,
            mask: 0b10000011,
        },
        ("rol", ZeroPage) => Op {
            code: 0x26,
            size: 2,
            cycles: 5,
            check: false,
            mask: 0b10000011,
        },
        ("rol", ZeroPageX) => Op {
            code: 0x36,
            size: 2,
            cycles: 6,
            check: false,
            mask: 0b10000011,
        },
        ("rol", Absolute) => Op {
            code: 0x2E,
            size: 3,
            cycles: 6,
            check: false,
            mask: 0b10000011,
        },
        ("rol", AbsoluteX) => Op {
            code: 0x3E,
            size: 3,
            cycles: 7,
            check: false,
            mask: 0b10000011,
        },
        ("ror", NoMode) => Op {
            code: 0x6A,
            size: 1,
            cycles: 2,
            check: false,
            mask: 0b10000011,
        },
        ("ror", ZeroPage) => Op {
            code: 0x66,
            size: 2,
            cycles: 5,
            check: false,
            mask: 0b10000011,
        },
        ("ror", ZeroPageX) => Op {
            code: 0x76,
            size: 2,
            cycles: 6,
            check: false,
            mask: 0b10000011,
        },
        ("ror", Absolute) => Op {
            code: 0x6E,
            size: 3,
            cycles: 6,
            check: false,
            mask: 0b10000011,
        },
        ("ror", AbsoluteX) => Op {
            code: 0x7E,
            size: 3,
            cycles: 7,
            check: false,
            mask: 0b10000011,
        },
        ("rti", NoMode) => Op {
            code: 0x40,
            size: 1,
            cycles: 6,
            check: false,
            mask: 0b11011111,
        },
        ("rts", NoMode) => Op {
            code: 0x60,
            size: 0,
            cycles: 6,
            check: false,
            mask: 0b00000000,
        },
        ("sbc", Immediate) => Op {
            code: 0xE9,
            size: 2,
            cycles: 2,
            check: false,
            mask: 0b11000011,
        },
        ("sbc", ZeroPage) => Op {
            code: 0xE5,
            size: 2,
            cycles: 3,
            check: false,
            mask: 0b11000011,
        },
        ("sbc", ZeroPageX) => Op {
            code: 0xF5,
            size: 2,
            cycles: 4,
            check: false,
            mask: 0b11000011,
        },
        ("sbc", Absolute) => Op {
            code: 0xED,
            size: 3,
            cycles: 4,
            check: false,
            mask: 0b11000011,
        },
        ("sbc", AbsoluteX) => Op {
            code: 0xFD,
            size: 3,
            cycles: 4,
            check: true,
            mask: 0b11000011,
        },
        ("sbc", AbsoluteY) => Op {
            code: 0xF9,
            size: 3,
            cycles: 4,
            check: true,
            mask: 0b11000011,
        },
        ("sbc", IndirectX) => Op {
            code: 0xE1,
            size: 2,
            cycles: 6,
            check: false,
            mask: 0b11000011,
        },
        ("sbc", IndirectY) => Op {
            code: 0xF1,
            size: 2,
            cycles: 5,
            check: true,
            mask: 0b11000011,
        },
        ("sec", NoMode) => Op {
            code: 0x38,
            size: 1,
            cycles: 2,
            check: false,
            mask: 0b00000001,
        },
        ("sed", NoMode) => Op {
            code: 0xF8,
            size: 1,
            cycles: 2,
            check: false,
            mask: 0b00001000,
        },
        ("sei", NoMode) => Op {
            code: 0x78,
            size: 1,
            cycles: 2,
            check: false,
            mask: 0b00000100,
        },
        ("sta", ZeroPage) => Op {
            code: 0x85,
            size: 2,
            cycles: 3,
            check: false,
            mask: 0b00000000,
        },
        ("sta", ZeroPageX) => Op {
            code: 0x95,
            size: 2,
            cycles: 4,
            check: false,
            mask: 0b00000000,
        },
        ("sta", Absolute) => Op {
            code: 0x8D,
            size: 3,
            cycles: 4,
            check: false,
            mask: 0b00000000,
        },
        ("sta", AbsoluteX) => Op {
            code: 0x9D,
            size: 3,
            cycles: 5,
            check: false,
            mask: 0b00000000,
        },
        ("sta", AbsoluteY) => Op {
            code: 0x99,
            size: 3,
            cycles: 5,
            check: false,
            mask: 0b00000000,
        },
        ("sta", IndirectX) => Op {
            code: 0x81,
            size: 2,
            cycles: 6,
            check: false,
            mask: 0b00000000,
        },
        ("sta", IndirectY) => Op {
            code: 0x91,
            size: 2,
            cycles: 6,
            check: false,
            mask: 0b00000000,
        },
        ("stx", ZeroPage) => Op {
            code: 0x86,
            size: 2,
            cycles: 3,
            check: false,
            mask: 0b00000000,
        },
        ("stx", ZeroPageY) => Op {
            code: 0x96,
            size: 2,
            cycles: 4,
            check: false,
            mask: 0b00000000,
        },
        ("stx", Absolute) => Op {
            code: 0x8E,
            size: 3,
            cycles: 4,
            check: false,
            mask: 0b00000000,
        },
        ("sty", ZeroPage) => Op {
            code: 0x84,
            size: 2,
            cycles: 3,
            check: false,
            mask: 0b00000000,
        },
        ("sty", ZeroPageX) => Op {
            code: 0x94,
            size: 2,
            cycles: 4,
            check: false,
            mask: 0b00000000,
        },
        ("sty", Absolute) => Op {
            code: 0x8C,
            size: 3,
            cycles: 4,
            check: false,
            mask: 0b00000000,
        },
        ("tax", NoMode) => Op {
            code: 0xAA,
            size: 1,
            cycles: 2,
            check: false,
            mask: 0b10000010,
        },
        ("tay", NoMode) => Op {
            code: 0xA8,
            size: 1,
            cycles: 2,
            check: false,
            mask: 0b10000010,
        },
        ("tsx", NoMode) => Op {
            code: 0xBA,
            size: 1,
            cycles: 2,
            check: false,
            mask: 0b10000010,
        },
        ("txa", NoMode) => Op {
            code: 0x8A,
            size: 1,
            cycles: 2,
            check: false,
            mask: 0b10000010,
        },
        ("txs", NoMode) => Op {
            code: 0x9A,
            size: 1,
            cycles: 2,
            check: false,
            mask: 0b00000000,
        },
        ("tya", NoMode) => Op {
            code: 0x98,
            size: 1,
            cycles: 2,
            check: false,
            mask: 0b10000010,
        },
        (_, _) => panic!("invalid instruction"),
    }
}
