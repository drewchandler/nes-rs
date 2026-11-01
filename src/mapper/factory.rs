use mapper::{Mapper, nrom::Nrom, unrom::Unrom, discrete::Discrete,
    mmc1::Mmc1, mmc2::Mmc2, mmc3::Mmc3, registers::Registers};
use rom::Rom;

pub const SUPPORTED: &[u16] = &[0, 1, 2, 3, 4, 7, 9, 10, 11, 13, 32, 34,
    38, 66, 70, 71, 75, 78, 79, 87, 89, 93, 94, 140, 152, 180, 206, 232];

// Both frontends use this board/layout validation before constructing a mapper.
// Memory-size and submapper limits prevent treating a distinct PCB as a merely
// larger version of a controller with different wiring.
pub fn supported(rom: &Rom) -> bool {
    let p = rom.prg_rom.len() * 16384;
    let c = rom.chr_rom.len() * 8192;
    if p == 0 || !p.is_power_of_two() || (c != 0 && !c.is_power_of_two()) ||
        rom.prg_rom.iter().any(|b| b.len() != 16384) ||
        rom.chr_rom.iter().any(|b| b.len() != 8192) ||
        (c != 0 && rom.chr_ram_size != 0) ||
        (c == 0 && !rom.chr_ram_size.is_power_of_two()) ||
        (rom.prg_ram_size != 0 && !rom.prg_ram_size.is_power_of_two()) { return false; }
    let range = |size: usize, low: usize, high: usize| size >= low * 1024 && size <= high * 1024;
    let ram = |kb: usize| c == 0 && rom.chr_ram_size == kb * 1024;
    let chr = |kb: usize| c != 0 && c <= kb * 1024;
    let no_prg_ram = !rom.nes2 || rom.prg_ram_size == 0;
    let small_ram = rom.prg_ram_size <= 8192;
    let s = rom.submapper;
    match rom.mapper {
        0 => s == 0 && p <= 32768 && (chr(8) || ram(8)) && no_prg_ram,
        1 => (s == 0 || (s == 5 && p == 32768 && no_prg_ram)) && range(p, 16, 512) &&
            ((chr(128) && small_ram) || (ram(8) && rom.prg_ram_size <= 32768)) &&
            (p <= 262144 || ram(8)),
        2 => s <= 2 && p <= 2097152 && ram(8) && no_prg_ram,
        3 => s <= 2 && p <= 32768 && chr(32) && small_ram,
        4 => [0, 1, 2, 4].contains(&s) && range(p, 32, 512) && (chr(256) || ram(8)) &&
            if s == 1 { rom.prg_ram_size == 1024 } else { small_ram },
        7 => s <= 2 && range(p, 32, 256) && ram(8) && no_prg_ram,
        9 => s == 0 && range(p, 32, 128) && chr(128) && small_ram,
        10 => s == 0 && range(p, 32, 256) && chr(128) && small_ram,
        11 => s <= 2 && range(p, 32, 128) && chr(128) && no_prg_ram,
        13 => s == 0 && p == 32768 && ram(16) && no_prg_ram,
        32 => s <= 1 && range(p, 32, 256) && (chr(256) || ram(8)) && small_ram,
        34 if s == 1 || (s == 0 && c != 0) => range(p, 32, 64) && chr(64) && small_ram,
        34 => (s == 0 || s == 2) && range(p, 32, 128) && ram(8) && no_prg_ram,
        38 => s == 0 && range(p, 32, 128) && chr(32) && no_prg_ram,
        66 => s == 0 && range(p, 32, 128) && chr(32) && no_prg_ram,
        70 | 89 | 140 | 152 => s == 0 && range(p, 32, 128) && chr(128) && no_prg_ram,
        71 => s <= 1 && range(p, 32, 256) && ram(8) && no_prg_ram,
        75 => s == 0 && range(p, 32, 128) && chr(128) && no_prg_ram,
        78 => ((!rom.nes2 && s == 0) || s == 1 || s == 3) &&
            range(p, 32, 128) && chr(128) && no_prg_ram,
        79 => s == 0 && range(p, 32, 64) && chr(64) && no_prg_ram,
        87 => s == 0 && p <= 32768 && chr(32) && no_prg_ram,
        93 | 94 => s == 0 && range(p, 32, 128) && ram(8) && no_prg_ram,
        180 => s == 0 && range(p, 16, 128) && ram(8) && no_prg_ram,
        206 => s == 0 && range(p, 32, 128) && chr(64) && no_prg_ram,
        232 => s == 0 && range(p, 64, 256) && ram(8) && no_prg_ram,
        _ => false,
    }
}

pub fn create(rom: Rom) -> Box<dyn Mapper> {
    assert!(supported(&rom), "Unsupported mapper {} / submapper {} / memory layout", rom.mapper, rom.submapper);
    match rom.mapper {
        0 => Box::new(Nrom::new(rom)),
        1 => Box::new(Mmc1::new(rom)),
        2 => Box::new(Unrom::new(rom)),
        4 | 206 => Box::new(Mmc3::new(rom)),
        9 | 10 => Box::new(Mmc2::new(rom)),
        32 | 75 => Box::new(Registers::new(rom)),
        _ => Box::new(Discrete::new(rom)),
    }
}
