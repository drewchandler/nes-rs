#[cfg(not(target_arch = "wasm32"))]
extern crate nes_rs;
#[cfg(not(target_arch = "wasm32"))]
fn main() {
    let mut args = std::env::args().skip(1);
    let filename = args
        .next()
        .expect("Usage: nes-rs ROM [--region ntsc|pal|dendy] [--controls FILE]");
    let mut rom = nes_rs::rom::Rom::load(&filename).expect("Could not read ROM");
    let mut controls = None;
    while let Some(option) = args.next() {
        let value = args.next().expect("Option needs a value");
        match option.as_str() {
            "--controls" => controls = Some(std::path::PathBuf::from(value)),
            "--region" => {
                rom.region = match value.as_str() {
                    "ntsc" => nes_rs::region::Region::Ntsc,
                    "pal" => nes_rs::region::Region::Pal,
                    "dendy" => nes_rs::region::Region::Dendy,
                    _ => panic!("Region must be ntsc, pal, or dendy"),
                }
            }
            _ => panic!("Unknown option: {}", option),
        }
    }
    let mut emulator =
        nes_rs::emulator::Emulator::new(rom, std::path::Path::new(&filename), controls.as_deref());
    emulator.run();
}
#[cfg(target_arch = "wasm32")]
fn main() {}
