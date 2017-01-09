use nes::Nes;
use rom::Rom;

pub struct Emulator {
    nes: Nes,
}

impl Emulator {
    pub fn new(rom: Rom) -> Emulator {
        Emulator { nes: Nes::new(rom) }
    }

    pub fn run(&mut self) {
        self.nes.reset();

        loop {
            self.nes.run_frame();
        }
    }
}
