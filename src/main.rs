mod rom;
mod emulator;

use std::env;

fn main() {
    let rom_filename = env::args().nth(1).unwrap();
    let rom = rom::Rom::load(rom_filename).unwrap();
    let emulator = emulator::Emulator::new(rom);
    emulator.run();
}
