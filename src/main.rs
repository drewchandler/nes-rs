extern crate minifb;

mod cpu;
mod emulator;
mod interconnect;
mod joypad;
mod mapper;
mod nes;
mod ppu;
mod rom;

use std::env;

fn main() {
    let rom_filename = env::args().nth(1).unwrap();
    let rom = rom::Rom::load(rom_filename).unwrap();
    let mut emulator = emulator::Emulator::new(rom);
    emulator.run();
}
