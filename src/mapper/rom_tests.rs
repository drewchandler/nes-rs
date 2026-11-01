use std::env;
use std::path::Path;
use interconnect::Interconnect;
use nes::Nes;
use rom::Rom;

fn run_rom(path: &'static str, submapper: u8) {
    std::thread::Builder::new().stack_size(8 * 1024 * 1024).spawn(move || {
        let root = env::var("NES_TEST_ROMS").expect("Set NES_TEST_ROMS; see docs/mappers.md");
        let mut rom = Rom::load(Path::new(&root).join(path)).unwrap();
        assert_eq!(rom.mapper, 4);
        rom.submapper = submapper; // Select the controller revision the test expects.
        let mut nes = Nes::new(rom);
        nes.reset();
        let mut instructions = 0;
        let mut running = false;
        while nes.interconnect.cpu_cycles < 20_000_000 {
            nes.cpu.step(&mut nes.interconnect);
            instructions += 1;
            if instructions % 256 != 0 { continue; }
            let bus = &mut nes.interconnect;
            if bus.read_word(0x6001) != 0xde || bus.read_word(0x6002) != 0xb0 || bus.read_word(0x6003) != 0x61 { continue; }
            let result = bus.read_word(0x6000);
            if result == 0x80 { running = true; }
            if running && result < 0x80 {
                let text: String = (0x6004..0x6400).map(|a| bus.read_word(a))
                    .take_while(|&b| b != 0).map(|b| b as char).collect();
                println!("{} (submapper {}): {}\n{}", path, submapper, result, text);
                assert_eq!(result, 0, "{}: {}", path, text);
                return;
            }
        }
        let bus = &mut nes.interconnect;
        let signature: Vec<u8> = (0x6000..0x6004).map(|a| bus.read_word(a)).collect();
        let text: String = (0x6004..0x6100).map(|a| bus.read_word(a))
            .take_while(|&b| b != 0).map(|b| b as char).collect();
        panic!("{} did not finish; RAM={:02x?}, text={}", path, signature, text);
    }).unwrap().join().unwrap();
}
macro_rules! fixture {
    ($name:ident, $path:expr, $revision:expr) => {
        #[test]
        #[ignore = "Requires public NES_TEST_ROMS fixtures; see docs/mappers.md"]
        fn $name() { run_rom($path, $revision); }
    };
}
fixture!(mmc3_clocking, "mmc3_test_2/rom_singles/1-clocking.nes", 0);
fixture!(mmc3_details, "mmc3_test_2/rom_singles/2-details.nes", 0);
fixture!(mmc3_a12_clocking, "mmc3_test_2/rom_singles/3-A12_clocking.nes", 0);
fixture!(mmc3_scanline_timing, "mmc3_test_2/rom_singles/4-scanline_timing.nes", 0);
fixture!(mmc3_sharp_revision, "mmc3_test_2/rom_singles/5-MMC3.nes", 0);
fixture!(mmc3_nec_revision, "mmc3_test_2/rom_singles/6-MMC3_alt.nes", 4);
