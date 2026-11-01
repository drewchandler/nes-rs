use super::*;
use interconnect::Interconnect;
use std::io::Cursor;

fn cartridge(mapper: u16, region: u8, battery: bool) -> Rom {
    let prg = if [0, 3, 13, 87].contains(&mapper) {
        32
    } else {
        128
    };
    let chr = if [0, 1, 2, 7, 13, 34, 71, 93, 94, 180, 232].contains(&mapper) {
        0
    } else {
        8
    };
    let mut bytes = vec![0; 16 + (prg + chr) * 1024];
    bytes[..10].copy_from_slice(&[
        0x4e,
        0x45,
        0x53,
        0x1a,
        (prg / 16) as u8,
        (chr / 8) as u8,
        ((mapper & 15) << 4) as u8,
        ((mapper & 0xf0) as u8) | 8,
        0,
        0,
    ]);
    if mapper == 78 {
        bytes[8] = 0x30;
    }
    bytes[10] = if battery { 0x70 } else { 0 };
    bytes[11] = if chr == 0 {
        if mapper == 13 {
            8
        } else {
            7
        }
    } else {
        0
    };
    bytes[12] = region;
    for bank in (0..prg * 1024).step_by(8192) {
        bytes[16 + bank..16 + bank + 6].copy_from_slice(&[0xe6, 0x10, 0x4c, 0, 0x80, 0xea]);
        bytes[16 + bank + 8188..16 + bank + 8190].copy_from_slice(&[0, 0x80]);
    }
    Rom::read(&mut Cursor::new(bytes)).unwrap()
}
#[test]
fn snapshots_restore_cpu_ppu_and_every_mapper_without_including_rom() {
    std::thread::Builder::new()
        .stack_size(12 * 1024 * 1024)
        .spawn(|| {
            for &id in ::mapper::SUPPORTED {
                let mut rom = cartridge(id, 0, false);
                if id == 79 {
                    rom.prg_rom.truncate(4);
                }
                if id == 87 {
                    rom.prg_rom.truncate(2);
                }
                let mut nes = Nes::new(rom);
                nes.reset();
                nes.run_frame_two(ButtonState::from_bits(8), ButtonState::from_bits(1));
                for _ in 0..37 {
                    nes.cpu.step(&mut nes.interconnect);
                }
                let saved = nes.save_state();
                assert!(saved.len() < 300_000, "ROM must stay outside the state");
                for _ in 0..2 {
                    nes.run_frame(ButtonState::default());
                }
                let expected = nes.save_state();
                nes.load_state(&saved).unwrap();
                assert_eq!(
                    nes.save_state(),
                    saved,
                    "mapper {} restores every serialized field",
                    id
                );
                for _ in 0..2 {
                    nes.run_frame(ButtonState::default());
                }
                assert_eq!(
                    nes.save_state(),
                    expected,
                    "mapper {} resumes deterministically",
                    id
                );
            }
        })
        .unwrap()
        .join()
        .unwrap();
}
#[test]
fn invalid_or_wrong_game_state_keeps_the_running_machine() {
    std::thread::Builder::new()
        .stack_size(12 * 1024 * 1024)
        .spawn(|| {
            let mut nes = Nes::new(cartridge(1, 0, true));
            nes.reset();
            nes.run_frame(ButtonState::default());
            let state = nes.save_state();
            let mut other = cartridge(1, 0, true);
            other.prg_rom[0][500] = 1;
            let wrong = Nes::new(other).save_state();
            assert!(nes.load_state(&wrong).is_err());
            assert_eq!(nes.save_state(), state);
            for length in [0, 8, 39, state.len() - 1] {
                assert!(nes.load_state(&state[..length]).is_err());
                assert_eq!(nes.save_state(), state);
            }
            let mut corrupt = state.clone();
            corrupt[50] ^= 0xff;
            assert!(nes.load_state(&corrupt).is_err());
            assert_eq!(nes.save_state(), state);
            let mut region = Nes::new(cartridge(1, 1, true));
            assert!(region.load_state(&state).is_err());
        })
        .unwrap()
        .join()
        .unwrap();
}
#[test]
fn battery_save_persists_only_nonvolatile_ram_and_rejects_wrong_sizes() {
    std::thread::Builder::new()
        .stack_size(12 * 1024 * 1024)
        .spawn(|| {
            let mut rom = cartridge(1, 0, true);
            rom.prg_ram_size = 16384;
            rom.prg_nvram_size = 8192;
            let mut nes = Nes::new(rom.clone());
            // SOROM: select RAM bank 0 (volatile), then bank 1 (nonvolatile).
            fn serial(n: &mut Nes, value: u8) {
                for bit in 0..5 {
                    n.interconnect.write_word(0xa000, (value >> bit) & 1);
                }
            }
            nes.interconnect.write_word(0x6000, 0x11);
            serial(&mut nes, 8);
            nes.interconnect.write_word(0x6000, 0x77);
            let save = nes.battery();
            assert_eq!(save.len(), 8192);
            assert_eq!(save[0], 0x77);
            let mut fresh = Nes::new(rom);
            fresh.load_battery(&save).unwrap();
            assert_eq!(fresh.interconnect.read_word(0x6000), 0);
            serial(&mut fresh, 8);
            assert_eq!(fresh.interconnect.read_word(0x6000), 0x77);
            assert!(fresh.load_battery(&save[..8191]).is_err());
            assert_eq!(fresh.battery(), save);
            assert!(Nes::new(cartridge(1, 0, false)).battery().is_empty());
        })
        .unwrap()
        .join()
        .unwrap();
}
#[test]
fn both_controller_ports_latch_independent_players_and_preserve_open_bus() {
    std::thread::Builder::new()
        .stack_size(12 * 1024 * 1024)
        .spawn(|| {
            let mut nes = Nes::new(cartridge(0, 0, false));
            let bus = &mut nes.interconnect;
            bus.joypad1.set_state(ButtonState::from_bits(0x08));
            bus.joypad2.set_state(ButtonState::from_bits(0x81));
            bus.write_word(0x4016, 1);
            bus.write_word(0x4016, 0);
            for bit in 0..10 {
                bus.write_word(0x5000, 0xa0);
                let first = bus.read_word(0x4016);
                bus.write_word(0x5000, 0xa0);
                let second = bus.read_word(0x4017);
                assert_eq!(first, 0xa0 | if bit < 8 { (0x08 >> bit) & 1 } else { 1 });
                assert_eq!(second, 0xa0 | if bit < 8 { (0x81 >> bit) & 1 } else { 1 });
            }
        })
        .unwrap()
        .join()
        .unwrap();
}
#[test]
fn regions_have_correct_cpu_to_frame_ratios_and_state_clock_phase() {
    std::thread::Builder::new()
        .stack_size(12 * 1024 * 1024)
        .spawn(|| {
            for &(header, ratio, lines) in &[(0, 3.0, 262.0), (1, 3.2, 312.0), (3, 3.0, 312.0)] {
                let mut nes = Nes::new(cartridge(0, header, false));
                nes.reset();
                nes.run_frame(ButtonState::default());
                let start = nes.interconnect.cpu_cycles;
                for _ in 0..10 {
                    nes.run_frame(ButtonState::default());
                }
                let cycles = (nes.interconnect.cpu_cycles - start) as f64;
                assert!(
                    (cycles - 341.0 * lines * 10.0 / ratio).abs() < 15.0,
                    "region {}: {}",
                    header,
                    cycles
                );
                assert!((nes.fps() - if header == 0 { 60.0988 } else { 50.007 }).abs() < 0.01);
                let saved = nes.save_state();
                nes.run_frame(ButtonState::default());
                let expected = nes.save_state();
                nes.load_state(&saved).unwrap();
                nes.run_frame(ButtonState::default());
                assert_eq!(nes.save_state(), expected);
            }
        })
        .unwrap()
        .join()
        .unwrap();
}

#[test]
fn identity_includes_rom_bank_boundaries_and_chr_nvram_is_persistent() {
    std::thread::Builder::new()
        .stack_size(12 * 1024 * 1024)
        .spawn(|| {
            let mut first = cartridge(1, 0, false);
            first.prg_rom = vec![vec![0; 16384]; 4];
            first.chr_rom = vec![vec![0; 8192]; 16];
            first.chr_ram_size = 0;
            let mut second = first.clone();
            second.prg_rom = vec![vec![0; 16384]; 8];
            second.chr_rom = vec![vec![0; 8192]; 8];
            assert_ne!(
                Nes::new(first).rom_id,
                Nes::new(second).rom_id,
                "equal concatenated bytes with different bank boundaries are distinct cartridges"
            );
            let mut rom = cartridge(0, 0, false);
            rom.chr_nvram_size = 8192;
            let mut game = Nes::new(rom.clone());
            game.interconnect.write_word(0x2006, 0);
            game.interconnect.write_word(0x2006, 0);
            game.interconnect.write_word(0x2007, 0xab);
            let save = game.battery();
            assert_eq!(save.len(), 8192);
            assert_eq!(save[0], 0xab);
            let mut restored = Nes::new(rom);
            restored.load_battery(&save).unwrap();
            restored.interconnect.write_word(0x2006, 0);
            restored.interconnect.write_word(0x2006, 0);
            restored.interconnect.read_word(0x2007);
            assert_eq!(restored.interconnect.read_word(0x2007), 0xab);
        })
        .unwrap()
        .join()
        .unwrap();
}
