use rom::Rom;

pub struct Emulator {
    rom: Rom,
}

impl Emulator {
    pub fn new(rom: Rom) -> Emulator {
        Emulator { rom: rom }
    }

    pub fn run(&self) {}
}
