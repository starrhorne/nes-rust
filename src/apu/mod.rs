mod frame_counter;
mod length_counter;
mod envelope;
mod pulse_channel;
mod triangle_channel;
mod noise_channel;
mod dmc_channel;
mod filter;
mod sequencer;
mod sweep;

use self::dmc_channel::DmcChannel;
use self::envelope::Envelope;
use self::filter::FirstOrderFilter;
use self::frame_counter::{FrameCounter, FrameResult};
use self::length_counter::LengthCounter;
use self::noise_channel::NoiseChannel;
use self::pulse_channel::PulseChannel;
use self::sequencer::Sequencer;
use self::sweep::{Sweep, SweepNegationMode};
use self::triangle_channel::TriangleChannel;

pub struct Apu {
    pub buffer: Vec<i16>,
    frame_counter: FrameCounter,
    pulse_0: PulseChannel,
    pulse_1: PulseChannel,
    triangle: TriangleChannel,
    noise: NoiseChannel,
    pub dmc: DmcChannel,
    filters: [FirstOrderFilter; 3],
}

impl Apu {
    pub fn new() -> Self {
        Apu {
            buffer: Vec::new(),
            frame_counter: FrameCounter::new(),
            pulse_0: PulseChannel::new(SweepNegationMode::OnesCompliment),
            pulse_1: PulseChannel::new(SweepNegationMode::TwosCompliment),
            triangle: TriangleChannel::new(),
            noise: NoiseChannel::new(),
            dmc: DmcChannel::new(),
            filters: [
                FirstOrderFilter::high_pass(44100.0, 90.0),
                FirstOrderFilter::high_pass(44100.0, 440.0),
                FirstOrderFilter::low_pass(44100.0, 14_000.0),
            ],
        }
    }

    pub fn reset(&mut self) {
        self.write_register(0x4017, 0, 0);
        for i in 0..11 {
            self.tick(i);
        }
    }

    pub fn read_register(&mut self) -> u8 {
        let mut result = 0;
        if self.dmc.irq_flag {
            result |= 0b1000_0000;
        }
        if self.frame_counter.private_irq_flag {
            result |= 0b0100_0000;
        }
        if self.dmc.playing() {
            result |= 0b0001_0000;
        }
        if self.noise.playing() {
            result |= 0b0000_1000;
        }
        if self.triangle.playing() {
            result |= 0b0000_0100;
        }
        if self.pulse_1.playing() {
            result |= 0b0000_0010;
        }
        if self.pulse_0.playing() {
            result |= 0b0000_0001;
        }

        self.frame_counter.private_irq_flag = false;
        self.frame_counter.public_irq_flag = false;
        result
    }

    pub fn write_register(&mut self, address: u16, value: u8, cycles: u64) {
        match address {
            0x4000...0x4003 => self.pulse_0.write_register(address, value),
            0x4004...0x4007 => self.pulse_1.write_register(address, value),
            0x4008...0x400B => self.triangle.write_register(address, value),
            0x400C...0x400F => self.noise.write_register(address, value),
            0x4010...0x4013 => self.dmc.write_register(address, value),
            0x4015 => {
                self.pulse_0.set_enabled(value & 0b0000_0001 != 0);
                self.pulse_1.set_enabled(value & 0b0000_0010 != 0);
                self.triangle.set_enabled(value & 0b0000_0100 != 0);
                self.noise.set_enabled(value & 0b0000_1000 != 0);
                self.dmc.set_enabled(value & 0b0001_0000 != 0);
            }
            0x4017 => {
                let r = self.frame_counter.write_register(value, cycles);
                self.handle_frame_result(r);
            }
            _ => panic!("Bad APU address: {:04X}", address),
        }
    }

    pub fn tick(&mut self, cpu_cycles: u64) {
        // Triangle ticks on each cpu cycle.
        self.triangle.tick_sequencer();

        // Everything else ticks on every other cycle
        if cpu_cycles % 2 == 1 {
            self.pulse_0.tick_sequencer();
            self.pulse_1.tick_sequencer();
            self.noise.tick_sequencer();
            self.dmc.tick_sequencer();
        }

        let r = self.frame_counter.tick();
        self.handle_frame_result(r);

        self.pulse_0.update_pending_length_counter();
        self.pulse_1.update_pending_length_counter();
        self.triangle.update_pending_length_counter();
        self.noise.update_pending_length_counter();

        // We need 730 stereo audio samples per frame for 60 fps.
        // Each frame lasts a minimum of 29,779 CPU cycles. This
        // works out to around 40 CPU cycles per sample.
        if cpu_cycles % 40 == 0 {
            let s = self.sample();
            self.buffer.push(s);
            self.buffer.push(s);
        }
    }

    fn handle_frame_result(&mut self, result: FrameResult) {
        match result {
            FrameResult::Quarter => {
                self.pulse_0.tick_quarter_frame();
                self.pulse_1.tick_quarter_frame();
                self.triangle.tick_quarter_frame();
            }
            FrameResult::Half => {
                self.pulse_0.tick_quarter_frame();
                self.pulse_0.tick_half_frame();
                self.pulse_1.tick_quarter_frame();
                self.pulse_1.tick_half_frame();
                self.triangle.tick_quarter_frame();
                self.triangle.tick_half_frame();
                self.noise.tick_quarter_frame();
                self.noise.tick_half_frame();
            }
            FrameResult::None => (),
        }
    }

    pub fn irq_flag(&self) -> bool {
        self.frame_counter.public_irq_flag || self.dmc.irq_flag
    }

    fn sample(&mut self) -> i16 {
        let p0 = self.pulse_0.sample() as f64;
        let p1 = self.pulse_1.sample() as f64;
        let t = self.triangle.sample() as f64;
        let n = self.noise.sample() as f64;
        let d = self.dmc.sample() as f64;

        // Combine channels into a single value from 0.0 to 1.0
        // Formula is from http://wiki.nesdev.com/w/index.php/APU_Mixer
        let pulse_out = 95.88 / ((8218.0 / (p0 + p1)) + 100.0);
        let tnd_out = 159.79 / ((1.0 / (t / 8227.0 + n / 12241.0 + d / 22638.0)) + 100.0);

        // Scale to 0..65536
        let mut output = (pulse_out + tnd_out) * 65535.0;

        // Apply high pass and low pass filters
        for i in 0..3 {
            output = self.filters[i].tick(output);
        }

        // The final range is -32767 to +32767
        output as i16
    }
}
