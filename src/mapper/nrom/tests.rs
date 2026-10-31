use super::*;
use interconnect::{Interconnect, MemoryMappingInterconnect};
use joypad::ButtonState;
use nes::Nes;
use rom::Mirroring;

fn cartridge(banks: usize, chr_rom: Vec<Vec<u8>>, mirroring: Mirroring) -> Rom {
    let mut prg_rom: Vec<Vec<u8>> = (0..banks)
        .map(|bank| vec![0x11 * (bank as u8 + 1); 16384]).collect();
    prg_rom[0][..3].copy_from_slice(&[0x4c, 0, 0x80]); // JMP $8000.
    prg_rom[banks - 1][0x3ffc..0x3ffe].copy_from_slice(&[0, 0x80]);
    Rom { prg_rom, chr_rom, mapper: 0, submapper: 0, mirroring }
}

fn set_ppu_addr(bus: &mut MemoryMappingInterconnect, addr: u16) {
    bus.read_word(0x2002);
    bus.write_word(0x2006, (addr >> 8) as u8);
    bus.write_word(0x2006, addr as u8);
}

fn read_ppu(bus: &mut MemoryMappingInterconnect, addr: u16) -> u8 {
    set_ppu_addr(bus, addr);
    bus.read_word(0x2007); // PPUDATA's delayed read.
    bus.read_word(0x2007)
}

#[test]
fn program_rom_maps_16k_mirroring_and_32k_banks_without_bank_registers() {
    for banks in 1..=2 {
        let mut mapper = Nrom::new(cartridge(banks, vec![], Mirroring::Horizontal));
        assert_eq!(mapper.read(0x9000), 0x11);
        assert_eq!(mapper.read(0xd000), if banks == 1 { 0x11 } else { 0x22 });
        assert_eq!(mapper.read(0xfffc), 0);
        assert_eq!(mapper.read(0xfffd), 0x80);
        mapper.write(0x9000, 0xff);
        mapper.write(0xd000, 0xff);
        assert_eq!(mapper.read(0x9000), 0x11);
        assert_eq!(mapper.read(0xd000), if banks == 1 { 0x11 } else { 0x22 });
    }
}

#[test]
fn cpu_ppu_ports_read_chr_rom_and_ignore_pattern_writes() {
    let mut chr = vec![0; 8192];
    chr[0] = 0xa5;
    chr[8191] = 0x5a;
    let mut bus = MemoryMappingInterconnect::new(cartridge(1, vec![chr], Mirroring::Horizontal));
    assert_eq!(bus.read_double(0xfffc), 0x8000);
    assert_eq!(read_ppu(&mut bus, 0), 0xa5);
    assert_eq!(read_ppu(&mut bus, 0x1fff), 0x5a);
    set_ppu_addr(&mut bus, 0);
    bus.write_word(0x2007, 0x33);
    set_ppu_addr(&mut bus, 0x1fff);
    bus.write_word(0x2007, 0x44);
    assert_eq!(read_ppu(&mut bus, 0), 0xa5);
    assert_eq!(read_ppu(&mut bus, 0x1fff), 0x5a);
}

#[test]
fn chr_ram_and_nametable_mirroring_survive_cartridge_routing() {
    for &mirroring in &[Mirroring::Horizontal, Mirroring::Vertical] {
        let mut bus = MemoryMappingInterconnect::new(cartridge(2, vec![], mirroring));
        set_ppu_addr(&mut bus, 0);
        bus.write_word(0x2007, 0xab);
        set_ppu_addr(&mut bus, 0x1fff);
        bus.write_word(0x2007, 0xcd);
        assert_eq!(read_ppu(&mut bus, 0), 0xab);
        assert_eq!(read_ppu(&mut bus, 0x1fff), 0xcd);
        set_ppu_addr(&mut bus, 0x2000);
        bus.write_word(0x2007, 0x42);
        let alias = if mirroring == Mirroring::Horizontal { 0x2400 } else { 0x2800 };
        assert_eq!(read_ppu(&mut bus, alias), 0x42);
    }
}

#[test]
fn real_cpu_frames_render_background_and_sprite_patterns_from_chr_rom() {
    std::thread::Builder::new().stack_size(8 * 1024 * 1024).spawn(|| {
        let mut chr = vec![0; 8192];
        for row in 0..8 {
            chr[row] = 0xff; // Background tile 0: color 1.
            chr[16 + row + 8] = 0xff; // Sprite tile 1: color 2.
        }
        let mut nes = Nes::new(cartridge(1, vec![chr], Mirroring::Vertical));
        nes.reset();
        let bus = &mut nes.interconnect;
        for &(addr, color) in &[(0x3f00, 0x0f), (0x3f01, 0x30), (0x3f12, 0x16)] {
            set_ppu_addr(bus, addr);
            bus.write_word(0x2007, color);
        }
        bus.write_word(0x2003, 0);
        for _ in 0..256 { bus.write_word(0x2004, 0xff); }
        bus.write_word(0x2003, 0);
        for &byte in &[39, 1, 0, 40] { bus.write_word(0x2004, byte); }
        set_ppu_addr(bus, 0x2000);
        bus.write_word(0x2005, 0);
        bus.write_word(0x2005, 0);
        bus.write_word(0x2001, 0x1e);
        for _ in 0..2 {
            nes.run_frame(ButtonState { a: false, b: false, select: false, start: false,
                up: false, down: false, left: false, right: false });
        }
        assert_eq!(nes.interconnect.ppu.screen[100 * 256 + 100], 0xffffff);
        assert_eq!(nes.interconnect.ppu.screen[40 * 256 + 40], 0xdb2b00);
    }).unwrap().join().unwrap();
}
