use nes::Nes;
use rom::Rom;
use minifb::{Window, WindowOptions, Key};

pub struct Emulator {
    nes: Nes,
    window: Window,
}

impl Emulator {
    pub fn new(rom: Rom) -> Emulator {
        Emulator {
            nes: Nes::new(rom),
            window: Window::new("NES", 256, 240, WindowOptions::default()).unwrap(),
        }
    }

    pub fn run(&mut self) {
        self.nes.reset();

        while self.window.is_open() {
            let frame = self.nes.run_frame();
            self.window.update_with_buffer(frame);
        }
    }
}
