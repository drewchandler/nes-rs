use joypad::ButtonState;
use nes::Nes;
use rom::Rom;
use minifb::{Window, WindowOptions, Key};
use audio::{AudioOutput, FramePacer};

pub struct Emulator {
    nes: Nes,
    window: Window,
    audio: Option<AudioOutput>,
}

impl Emulator {
    pub fn new(rom: Rom) -> Emulator {
        let audio = match AudioOutput::new() {
            Ok(output) => Some(output),
            Err(error) => {
                eprintln!("Audio unavailable: {}. Continuing without sound.", error);
                None
            }
        };
        Emulator {
            nes: Nes::new(rom),
            window: Window::new("NES", 256, 240, WindowOptions::default()).unwrap(),
            audio,
        }
    }

    pub fn run(&mut self) {
        self.nes.reset();
        let mut pacer = FramePacer::new();

        while self.window.is_open() {
            let joypad1_state = ButtonState {
                a: self.window.is_key_down(Key::Z),
                b: self.window.is_key_down(Key::X),
                select: self.window.is_key_down(Key::RightShift),
                start: self.window.is_key_down(Key::Enter),
                up: self.window.is_key_down(Key::Up),
                down: self.window.is_key_down(Key::Down),
                left: self.window.is_key_down(Key::Left),
                right: self.window.is_key_down(Key::Right),
            };

            let frame = self.nes.run_frame(joypad1_state);
            self.window.update_with_buffer(frame);
            let samples = self.nes.interconnect.apu.take_samples();
            let error = self.audio.as_mut().and_then(|audio| audio.submit(&samples).err());
            if let Some(error) = error {
                eprintln!("Audio unavailable: {}. Continuing without sound.", error);
                self.audio = None;
                pacer = FramePacer::new();
            }
            if self.audio.is_none() { pacer.wait(); }
        }
    }
}
