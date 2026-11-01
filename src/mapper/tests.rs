use super::*;
use rom::{Mirroring, Rom};
use interconnect::{Interconnect, MemoryMappingInterconnect};

fn image(id: u16, prg_kb: usize, chr_kb: usize, chr_ram: usize, ram: usize, sub: u8) -> Rom {
    let mut prg_rom = vec![vec![0; 16384]; prg_kb / 16];
    for (bank, bytes) in prg_rom.iter_mut().enumerate() {
        for offset in 0..16384 { bytes[offset] = (bank * 2 + offset / 8192) as u8; }
        bytes[0] = 255;
        bytes[8192] = 255; // Bus-conflict-safe register addresses for banking tests.
    }
    let mut chr_rom = vec![vec![0; 8192]; chr_kb / 8];
    for (bank, bytes) in chr_rom.iter_mut().enumerate() {
        for offset in 0..8192 { bytes[offset] = (bank * 8 + offset / 1024) as u8; }
    }
    Rom { prg_rom, chr_rom, mapper: id, submapper: sub, mirroring: Mirroring::Vertical,
        prg_ram_size: ram, chr_ram_size: chr_ram, nes2: true }
}
fn make(id: u16) -> Rom {
    match id {
        0 => image(id, 32, 8, 0, 0, 0),
        1 => image(id, 256, 128, 0, 8192, 0),
        2 => image(id, 128, 0, 8192, 0, 0),
        3 => image(id, 32, 32, 0, 0, 1),
        4 => image(id, 128, 128, 0, 8192, 0),
        7 => image(id, 256, 0, 8192, 0, 1),
        9 => image(id, 128, 128, 0, 8192, 0),
        10 => image(id, 256, 128, 0, 8192, 0),
        11 | 70 | 75 | 89 | 140 | 152 => image(id, 128, 128, 0, 0, 0),
        13 => image(id, 32, 0, 16384, 0, 0),
        32 => image(id, 128, 128, 0, 8192, 0),
        34 => image(id, 128, 0, 8192, 0, 2),
        38 | 66 => image(id, 128, 32, 0, 0, 0),
        71 | 232 => image(id, 256, 0, 8192, 0, 0),
        78 => image(id, 128, 128, 0, 0, 3),
        79 => image(id, 64, 64, 0, 0, 0),
        87 => image(id, 32, 32, 0, 0, 0),
        93 | 94 | 180 => image(id, 128, 0, 8192, 0, 0),
        206 => image(id, 128, 64, 0, 0, 0),
        _ => panic!("missing test profile"),
    }
}
fn serial(m: &mut dyn Mapper, addr: u16, value: u8) {
    for bit in 0..5 { m.write(addr, (value >> bit) & 1); }
}

#[test]
fn supported_profiles_construct_and_unknown_hardware_is_rejected() {
    for &id in SUPPORTED {
        let rom = make(id);
        assert!(supported(&rom), "mapper {}", id);
        let m = create(rom);
        m.read(0x8100);
        m.read_chr(0x0100);
    }
    let mut rom = make(0);
    rom.mapper = 5;
    assert!(!supported(&rom));
    rom = make(3); rom.submapper = 15; assert!(!supported(&rom));
    rom = make(4); rom.submapper = 3; assert!(!supported(&rom)); // MC-ACC needs falling-edge IRQs.
    rom = make(1); rom.chr_ram_size = 8192; assert!(!supported(&rom));
    rom = make(1); rom.prg_ram_size = 32768; assert!(!supported(&rom));
    rom = make(0); rom.prg_ram_size = 65536; assert!(!supported(&rom));
}

#[test]
fn discrete_registers_follow_each_boards_prg_and_chr_bit_wiring() {
    // id, register, data, PRG signature at $8100/$C100, CHR signature at $0100.
    for &(id, addr, data, low, high, chr) in &[
        (3, 0x8000, 3, 0, 2, 24), (7, 0x8000, 0x12, 8, 10, 0),
        (11, 0x8000, 0x31, 4, 6, 24), (34, 0x8000, 2, 8, 10, 0),
        (38, 0x7000, 9, 4, 6, 16), (66, 0x8000, 0x23, 8, 10, 24),
        (70, 0x8000, 0x52, 10, 14, 16), (71, 0xc000, 5, 10, 30, 0),
        (78, 0x8000, 0x3a, 4, 14, 24), (79, 0x4100, 0x0d, 4, 6, 40),
        (87, 0x6000, 1, 0, 2, 16), (89, 0x8000, 0xab, 4, 14, 88),
        (93, 0x8000, 0x51, 10, 14, 0), (94, 0x8000, 0x14, 10, 14, 0),
        (140, 0x6000, 0x23, 8, 10, 24), (152, 0x8000, 0xd3, 10, 14, 24),
        (180, 0x8000, 3, 0, 6, 0), (232, 0xc000, 2, 4, 6, 0),
    ] {
        let mut m = create(make(id));
        if addr < 0x8000 { m.write_low(addr, data); } else { m.write(addr, data); }
        assert_eq!(m.read(0x8100), low, "mapper {} lower PRG", id);
        assert_eq!(m.read(0xc100), high, "mapper {} upper PRG", id);
        assert_eq!(m.read_chr(0x0100), Some(chr), "mapper {} CHR", id);
    }
}

#[test]
fn low_register_address_decoders_ignore_unrelated_cpu_writes() {
    for &(id, wrong, right, value, expected) in &[
        (38, 0x6000, 0x7000, 4, 8), (79, 0x4000, 0x4100, 1, 8),
        (87, 0x5000, 0x6000, 1, 16), (140, 0x5000, 0x6000, 1, 8),
    ] {
        let mut m = create(make(id));
        m.write_low(wrong, value); assert_eq!(m.read_chr(0), Some(0));
        m.write_low(right, value); assert_eq!(m.read_chr(0), Some(expected));
    }
}

#[test]
fn bus_conflicts_use_the_current_rom_byte_and_submapper_policy() {
    for &(sub, expected) in &[(0, 8), (1, 24), (2, 8)] {
        let mut rom = make(3); rom.submapper = sub; rom.prg_rom[0][0] = 1;
        let mut m = create(rom); m.write(0x8000, 3);
        assert_eq!(m.read_chr(0), Some(expected));
    }
    let mut rom = make(7); rom.submapper = 2; rom.prg_rom[0][0] = 1;
    let mut m = create(rom); m.write(0x8000, 2); assert_eq!(m.read(0x8100), 0);
}

#[test]
fn cprom_keeps_lower_ram_fixed_and_banks_upper_ram_independently() {
    let mut m = create(make(13));
    m.write_chr(0, 0xa1);
    m.write(0x8000, 1); m.write_chr(0x1000, 0xb2);
    m.write(0x8000, 2); m.write_chr(0x1000, 0xc3);
    assert_eq!(m.read_chr(0), Some(0xa1));
    m.write(0x8000, 1); assert_eq!(m.read_chr(0x1000), Some(0xb2));
    m.write(0x8000, 2); assert_eq!(m.read_chr(0x1000), Some(0xc3));
    m.write(0x8000, 0); assert_eq!(m.read_chr(0x1000), Some(0xa1));
}

#[test]
fn nina001_and_quattro_have_separate_registers_and_outer_banks() {
    let mut m = create(image(34, 64, 64, 0, 8192, 1));
    m.write_low(0x6000, 0xaa); m.write_low(0x7ffd, 1);
    m.write_low(0x7ffe, 2); m.write_low(0x7fff, 3);
    assert_eq!(m.read(0x8100), 4); assert_eq!(m.read(0xc100), 6);
    assert_eq!(m.read_chr(0x0100), Some(8)); assert_eq!(m.read_chr(0x1100), Some(12));
    assert_eq!(m.read_low(0x6000), Some(0xaa));
    m.write(0x8000, 0); assert_eq!(m.read(0x8100), 4);
    let mut q = create(make(232));
    q.write(0x8000, 0x18); q.write(0xc000, 2);
    assert_eq!(q.read(0x8100), 28); assert_eq!(q.read(0xc100), 30);
    q.write(0x8000, 8); assert_eq!(q.read(0x8100), 12); assert_eq!(q.read(0xc100), 14);
}

#[test]
fn mirroring_bits_and_legacy_variant_selection_match_the_board() {
    for &(id, value, expected) in &[(7, 0x10, Mirroring::SingleUpper),
        (78, 8, Mirroring::Vertical), (89, 8, Mirroring::SingleUpper),
        (152, 0x80, Mirroring::SingleUpper)] {
        let mut m = create(make(id)); m.write(0x8000, value); assert_eq!(m.mirroring(), Some(expected));
    }
    let mut rom = make(78); rom.submapper = 1;
    let mut m = create(rom); m.write(0x8000, 8); assert_eq!(m.mirroring(), Some(Mirroring::SingleUpper));
    let mut rom = make(71); rom.nes2 = false;
    let mut m = create(rom); m.write(0x8000, 0); assert_eq!(m.mirroring(), None);
    m.write(0x9000, 0x10); assert_eq!(m.mirroring(), Some(Mirroring::SingleUpper));
}

#[test]
fn mmc1_serial_modes_chr_banks_and_ram_disable_work_together() {
    let mut m = create(make(1));
    serial(&mut *m, 0xe000, 3); assert_eq!(m.read(0x8100), 6); assert_eq!(m.read(0xc100), 30);
    serial(&mut *m, 0x8000, 0x1f); serial(&mut *m, 0xa000, 3); serial(&mut *m, 0xc000, 5);
    assert_eq!(m.read_chr(0x0100), Some(12)); assert_eq!(m.read_chr(0x1100), Some(20));
    assert_eq!(m.mirroring(), Some(Mirroring::Horizontal));
    serial(&mut *m, 0x8000, 0x1a); assert_eq!(m.read(0x8100), 0); assert_eq!(m.read(0xc100), 6);
    serial(&mut *m, 0x8000, 3); assert_eq!(m.read(0x8100), 4); assert_eq!(m.read(0xc100), 6);
    assert_eq!(m.read_chr(0x0100), Some(8)); assert_eq!(m.read_chr(0x1100), Some(12));
    m.write_low(0x6000, 0xaa); assert_eq!(m.read_low(0x6000), Some(0xaa));
    serial(&mut *m, 0xe000, 0x10); assert_eq!(m.read_low(0x6000), None);
    m.write_low(0x6000, 0xbb); serial(&mut *m, 0xe000, 0); assert_eq!(m.read_low(0x6000), Some(0xaa));
    m.write(0x8000, 0x80); assert_eq!(m.read(0xc100), 30);
}

#[test]
fn mmc1_ignores_consecutive_cycle_data_writes_and_supports_outer_memory_banks() {
    let mut m = create(make(1));
    for bit in 0..5 {
        let cycle = 100 + bit as u64 * 3;
        m.write_cpu(0xe000, (5 >> bit) & 1, cycle);
        m.write_cpu(0xe000, 0, cycle + 1);
    }
    assert_eq!(m.read(0x8100), 10);
    let mut m = create(image(1, 512, 0, 8192, 32768, 0));
    serial(&mut *m, 0xa000, 0x10); serial(&mut *m, 0xe000, 3);
    assert_eq!(m.read(0x8100), 38); assert_eq!(m.read(0xc100), 62);
    serial(&mut *m, 0xa000, 0); m.write_low(0x6000, 0xaa);
    serial(&mut *m, 0xa000, 4); m.write_low(0x6000, 0xbb);
    serial(&mut *m, 0xa000, 0); assert_eq!(m.read_low(0x6000), Some(0xaa));
    serial(&mut *m, 0xa000, 4); assert_eq!(m.read_low(0x6000), Some(0xbb));
}

#[test]
fn mmc2_and_mmc4_latches_switch_after_the_triggering_pattern_read() {
    for id in [9, 10] {
        let mut m = create(make(id));
        for &(addr, value) in &[(0xb000, 2), (0xc000, 3), (0xd000, 4), (0xe000, 5)] { m.write(addr, value); }
        assert_eq!(m.read_chr(0x0100), Some(12));
        assert_eq!(m.read_chr(0x0fd8), Some(15)); assert_eq!(m.read_chr(0x0100), Some(8));
        m.read_chr(0x0fe8); assert_eq!(m.read_chr(0x0100), Some(12));
        m.read_chr(0x0fd9); assert_eq!(m.read_chr(0x0100), Some(if id == 9 { 12 } else { 8 }));
        assert_eq!(m.read_chr(0x1fd8), Some(23)); assert_eq!(m.read_chr(0x1100), Some(16));
        m.read_chr(0x1fe9); assert_eq!(m.read_chr(0x1100), Some(20));
        m.write(0xa000, 3); assert_eq!(m.read(0x8100), if id == 9 { 3 } else { 6 });
        assert_eq!(m.read(0xe100), if id == 9 { 15 } else { 31 });
    }
}

#[test]
fn mmc3_prg_and_chr_inversion_ram_protection_and_four_screen_override() {
    let mut m = create(make(4));
    m.write(0x8000, 6); m.write(0x8001, 5); assert_eq!(m.read(0x8100), 5);
    m.write(0x8000, 0x46); assert_eq!(m.read(0x8100), 14); assert_eq!(m.read(0xc100), 5);
    for &(reg, value) in &[(0, 3), (1, 7), (2, 12)] { m.write(0x8000, reg); m.write(0x8001, value); }
    assert_eq!(m.read_chr(0x0100), Some(2)); assert_eq!(m.read_chr(0x0500), Some(3));
    assert_eq!(m.read_chr(0x0900), Some(6)); assert_eq!(m.read_chr(0x1100), Some(12));
    m.write(0x8000, 0x80); assert_eq!(m.read_chr(0x0100), Some(12)); assert_eq!(m.read_chr(0x1100), Some(2));
    m.write(0xa001, 0x80); m.write_low(0x6000, 0xaa);
    m.write(0xa001, 0xc0); m.write_low(0x6000, 0xbb); assert_eq!(m.read_low(0x6000), Some(0xaa));
    m.write(0xa001, 0); assert_eq!(m.read_low(0x6000), None);
    let mut rom = make(4); rom.mirroring = Mirroring::FourScreen;
    let mut m = create(rom); m.write(0xa000, 1); assert_eq!(m.mirroring(), Some(Mirroring::FourScreen));
}

#[test]
fn mmc6_protects_two_ram_halves_and_mirrors_one_kibibyte() {
    let mut m = create(image(4, 128, 128, 0, 1024, 1));
    m.write(0x8000, 0x20); m.write(0xa001, 0xf0);
    m.write_low(0x7000, 0xaa); m.write_low(0x7200, 0xbb);
    assert_eq!(m.read_low(0x7400), Some(0xaa)); assert_eq!(m.read_low(0x7600), Some(0xbb));
    m.write(0xa001, 0xa0); m.write_low(0x7000, 0x33); m.write_low(0x7200, 0x44);
    assert_eq!(m.read_low(0x7000), Some(0xaa)); assert_eq!(m.read_low(0x7200), Some(0xbb));
    assert_eq!(m.read_low(0x6000), None);
    m.write(0xa001, 0x30);
    assert_eq!(m.read_low(0x7200), Some(0), "a disabled half reads zero while the other drives the bus");
    m.write(0xa001, 0x50); // Write-enable without read-enable does not permit writes.
    m.write_low(0x7000, 0x33); m.write_low(0x7200, 0x44);
    assert_eq!(m.read_low(0x7000), None);
    m.write(0xa001, 0xa0);
    assert_eq!(m.read_low(0x7000), Some(0xaa)); assert_eq!(m.read_low(0x7200), Some(0xbb));
    m.write(0x8000, 0); m.write(0xa001, 0xf0);
    m.write(0x8000, 0x20);
    assert_eq!(m.read_low(0x7000), None, "global disable clears protection; disabled writes cannot restore it");
}

#[test]
fn mmc3_irq_filters_short_a12_pulses_and_can_be_acknowledged() {
    let mut m = create(make(4)); m.write(0xc000, 2); m.write(0xc001, 0); m.write(0xe001, 0);
    for &(low, high, expected) in &[(0, 8, false), (10, 14, false), (20, 28, false), (30, 38, true)] {
        m.ppu_address(0, low); m.ppu_address(0x1000, high); assert_eq!(m.irq(), expected);
    }
    m.write(0xe000, 0); assert!(!m.irq());
    let mut sharp = create(make(4)); sharp.write(0xc000, 0); sharp.write(0xe001, 0);
    sharp.ppu_address(0, 0); sharp.ppu_address(0x1000, 8); assert!(sharp.irq());
    let mut rom = make(4); rom.submapper = 4;
    let mut nec = create(rom); nec.write(0xc000, 0); nec.write(0xe001, 0);
    nec.ppu_address(0, 0); nec.ppu_address(0x1000, 8); assert!(!nec.irq());
    nec.write(0xc001, 0); nec.ppu_address(0, 10); nec.ppu_address(0x1000, 18);
    assert!(nec.irq(), "explicit reload-to-zero asserts on NEC chips");
}

#[test]
fn dxrom_g101_and_vrc1_preserve_their_distinct_register_layouts() {
    let mut dx = create(make(206)); dx.write(0x8000, 0xc6); dx.write(0x8001, 5);
    assert_eq!(dx.read(0x8100), 5); assert_eq!(dx.read(0xc100), 14);
    dx.write(0xc000, 0); dx.write(0xe001, 0); dx.ppu_address(0x1000, 20); assert!(!dx.irq());
    let mut g = create(make(32)); g.write(0x8000, 5); g.write(0xa000, 7); g.write(0x9000, 3);
    assert_eq!(g.read(0x8100), 14); assert_eq!(g.read(0xa100), 7); assert_eq!(g.read(0xc100), 5);
    g.write(0xb003, 17); assert_eq!(g.read_chr(0x0d00), Some(17));
    let mut v = create(make(75)); v.write(0x8000, 3); v.write(0xa000, 4); v.write(0xc000, 5);
    assert_eq!(v.read(0x8100), 3); assert_eq!(v.read(0xa100), 4); assert_eq!(v.read(0xc100), 5);
    v.write(0xe000, 2); v.write(0x9000, 2); assert_eq!(v.read_chr(0), Some(72));
    v.write(0xf000, 3); v.write(0x9000, 6); assert_eq!(v.read_chr(0x1000), Some(76));
}

#[test]
fn ppu_dot_clock_drives_mapper_irq_into_the_cpu_bus() {
    let mut bus = MemoryMappingInterconnect::new(make(4));
    bus.write_word(0x2000, 8); bus.write_word(0x2001, 0x18);
    bus.write_word(0xc000, 2); bus.write_word(0xc001, 0); bus.write_word(0xe001, 0);
    for _ in 0..1200 {
        bus.cpu_read(0xe100);
        if bus.poll_irq() {
            bus.write_word(0xe000, 0); bus.cpu_read(0xe100);
            assert!(!bus.poll_irq()); return;
        }
    }
    panic!("PPU pattern fetches did not clock the MMC3 IRQ");
}

#[test]
fn mmc1_reset_on_a_consecutive_write_clears_partial_serial_data() {
    let mut m = create(make(1));
    m.write_cpu(0xe000, 1, 100);
    m.write_cpu(0xe000, 0x80, 101); // An RMW can set D7 on its second write.
    for bit in 0..5 { m.write_cpu(0xe000, (2 >> bit) & 1, 110 + bit * 3); }
    assert_eq!(m.read(0x8100), 4);
    m.write_cpu(0x8000, 0x80, 200); // Bill & Ted: INC $FF resets, then writes $00.
    m.write_cpu(0x8000, 0, 201);
    for bit in 0..5 { m.write_cpu(0xe000, (3 >> bit) & 1, 210 + bit * 3); }
    assert_eq!(m.read(0x8100), 6);
}

#[test]
fn cpu_bank_registers_change_ppu_nametable_aliases_without_erasing_ram() {
    fn write_table(bus: &mut MemoryMappingInterconnect, high: u8, value: u8) {
        bus.write_word(0x2006, high); bus.write_word(0x2006, 0); bus.write_word(0x2007, value);
    }
    fn read_table(bus: &mut MemoryMappingInterconnect, high: u8) -> u8 {
        bus.write_word(0x2006, high); bus.write_word(0x2006, 0);
        bus.read_word(0x2007); bus.read_word(0x2007)
    }
    let mut bus = MemoryMappingInterconnect::new(make(7));
    write_table(&mut bus, 0x20, 0xaa);
    for high in [0x20, 0x24, 0x28, 0x2c] { assert_eq!(read_table(&mut bus, high), 0xaa); }
    bus.write_word(0x8000, 0x10); write_table(&mut bus, 0x28, 0xbb);
    for high in [0x20, 0x24, 0x28, 0x2c] { assert_eq!(read_table(&mut bus, high), 0xbb); }
    bus.write_word(0x8000, 0); assert_eq!(read_table(&mut bus, 0x20), 0xaa);
}

#[test]
fn banked_chr_rom_is_read_only_and_sunsoft93_disabled_ram_floats_the_bus() {
    let mut m = create(make(4)); m.write(0x8000, 2); m.write(0x8001, 12);
    m.write_chr(0x1000, 255); assert_eq!(m.read_chr(0x1000), Some(12));
    let mut disabled = create(make(93));
    assert_eq!(disabled.read_chr(0), None, "disabled CHR RAM leaves the PPU bus floating");
    disabled.write(0x8000, 1); assert_eq!(disabled.read_chr(0), Some(0));
    let mut bus = MemoryMappingInterconnect::new(make(93));
    bus.write_word(0x8000, 1);
    bus.write_word(0x2006, 0); bus.write_word(0x2006, 0); bus.write_word(0x2007, 0xab);
    bus.write_word(0x8000, 0);
    bus.write_word(0x2006, 0); bus.write_word(0x2006, 0); bus.write_word(0x2007, 0xcd);
    bus.write_word(0x8000, 1);
    bus.write_word(0x2006, 0); bus.write_word(0x2006, 0);
    bus.read_word(0x2007); assert_eq!(bus.read_word(0x2007), 0xab, "disabled writes preserve RAM");
}
