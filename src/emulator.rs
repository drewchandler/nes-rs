use joypad::ButtonState;
use minifb::{Key, Window, WindowOptions};
use nes::Nes;
use rom::Rom;

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
        }
    }
}
