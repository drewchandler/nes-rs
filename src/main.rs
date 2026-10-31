#[cfg(not(target_arch = "wasm32"))]
extern crate nes_rs;

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    let rom_filename = std::env::args().nth(1).expect("Usage: nes-rs <ROM>");
    let rom = nes_rs::rom::Rom::load(rom_filename).unwrap();
    let mut emulator = nes_rs::emulator::Emulator::new(rom);
    emulator.run();
}

#[cfg(target_arch = "wasm32")]
fn main() {}
