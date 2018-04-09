#[macro_use]
extern crate bitfield;

#[macro_use]
extern crate libretro_backend;
use libretro_backend::{AudioVideoInfo, CoreInfo, GameData, JoypadButton, LoadGameResult,
                       PixelFormat, Region, RuntimeHandle};

extern crate rand;
extern crate time;

mod cpu;
mod cpu_debug;
mod apu;
mod ppu;
mod bus;
mod cartridge;
mod controller;

use bus::Bus;
use cpu::Cpu;
use controller::Button;

struct NesCore {
    game_data: Option<GameData>,
    cpu: Cpu,
    frame_count: i32,
    frame_second: i32,
}

impl NesCore {
    fn new() -> NesCore {
        NesCore {
            game_data: None,
            cpu: Cpu::new(Bus::new()),
            frame_count: 0,
            frame_second: 0,
        }
    }
}

impl Default for NesCore {
    fn default() -> Self {
        Self::new()
    }
}

impl libretro_backend::Core for NesCore {
    fn info() -> CoreInfo {
        CoreInfo::new("NES", env!("CARGO_PKG_VERSION")).supports_roms_with_extension("nes")
    }

    fn on_load_game(&mut self, game_data: GameData) -> LoadGameResult {
        if game_data.is_empty() {
            return LoadGameResult::Failed(game_data);
        }

        let result: Result<(), ()> = if let Some(data) = game_data.data() {
            self.cpu.bus.load_rom_from_memory(data);
            self.cpu.reset();
            self.cpu.bus.reset();
            Ok(())
        } else {
            panic!("Loading roms from files not supported")
        };

        match result {
            Ok(_) => {
                let av_info = AudioVideoInfo::new()
                    .video(256, 240, 60.0, PixelFormat::ARGB8888)
                    .audio(44100.0)
                    .region(Region::NTSC);

                self.game_data = Some(game_data);
                LoadGameResult::Success(av_info)
            }
            Err(_) => LoadGameResult::Failed(game_data),
        }
    }

    fn on_unload_game(&mut self) -> GameData {
        self.game_data.take().unwrap()
    }

    fn on_run(&mut self, handle: &mut RuntimeHandle) {
        macro_rules! update_controllers {
            ( $( $button:ident ),+ ) => (
                $(
                    self.cpu.bus.controller_0.set_button_state(Button::$button, handle.is_joypad_button_pressed( 0, JoypadButton::$button ));
                    self.cpu.bus.controller_1.set_button_state(Button::$button, handle.is_joypad_button_pressed( 1, JoypadButton::$button ));
                )+
            )
        }

        update_controllers!(A, B, Start, Select, Left, Up, Right, Down);

        let second = time::now().tm_sec;
        if self.frame_second != second {
            self.frame_count = 0;
            self.frame_second = second;
        }

        while !self.cpu.bus.draw {
            self.cpu.execute_next_instruction();
            let stall_cycles = self.cpu.bus.reset_cpu_stall_cycles();
            for _ in 0..stall_cycles {
                self.cpu.bus.tick()
            }
        }

        self.cpu.bus.draw = false;

        let mut video_frame = [0u8; 256 * 240 * 4];

        for i in 0..video_frame.len() {
            let pixel = self.cpu.bus.ppu.renderer.pixels[i / 4];
            video_frame[i] = (pixel >> (i % 4 * 8)) as u8;
        }

        handle.upload_video_frame(&video_frame);

        let audio_buffer_size = self.cpu.bus.apu.buffer.len();
        if audio_buffer_size < 1470 {
            for _ in 0..1470 - audio_buffer_size {
                self.cpu.bus.apu.buffer.push(0);
            }
        }
        handle.upload_audio_frame(&self.cpu.bus.apu.buffer[..]);
        self.cpu.bus.apu.buffer.clear();

        self.frame_count += 1;
    }

    fn on_reset(&mut self) {
        self.cpu.bus.reset();
        self.cpu.reset();
    }
}

libretro_core!(NesCore);
