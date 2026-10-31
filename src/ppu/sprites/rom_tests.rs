// Optional hardware-tested ROMs from christopherpow/nes-test-roms. Set
// NES_TEST_ROMS to a checkout; see docs/sprite-rendering.md for reproduction.
use cpu::Cpu;
use interconnect::{Interconnect, MemoryMappingInterconnect};
use rom::Rom;
use std::env;
use std::path::Path;

struct RomBus {
    inner: MemoryMappingInterconnect,
    reads: usize,
    addresses: [u16; 3],
    values: [u8; 3],
}

impl Interconnect for RomBus {
    fn read_word(&mut self, addr: u16) -> u8 {
        let value = self.inner.read_word(addr);
        if self.reads < 3 {
            self.addresses[self.reads] = addr;
            self.values[self.reads] = value;
        }
        self.reads += 1;
        value
    }
    fn write_word(&mut self, addr: u16, value: u8) {
        self.inner.write_word(addr, value);
    }
    fn read_double(&mut self, addr: u16) -> u16 {
        let low = self.read_word(addr) as u16;
        low | ((self.read_word(addr.wrapping_add(1)) as u16) << 8)
    }
    fn write_double(&mut self, addr: u16, value: u16) {
        self.write_word(addr, value as u8);
        self.write_word(addr.wrapping_add(1), (value >> 8) as u8);
    }
    fn cpu_read(&mut self, addr: u16) -> u8 {
        let value = self.inner.cpu_read(addr);
        if self.reads < 3 {
            self.addresses[self.reads] = addr;
            self.values[self.reads] = value;
        }
        self.reads += 1;
        value
    }
    fn cpu_write(&mut self, addr: u16, value: u8) { self.inner.cpu_write(addr, value); }
    fn oam_dma_read(&mut self, addr: u16) -> u8 { self.inner.oam_dma_read(addr) }
    fn take_stall_cycles(&mut self) -> u16 { self.inner.take_stall_cycles() }
    fn poll_irq(&self) -> bool { self.inner.poll_irq() }
    fn take_nmi(&mut self) -> bool { self.inner.take_nmi() }
    fn reset_cpu(&mut self) { self.inner.reset_cpu(); }
}

impl RomBus {
    fn halted(&self, cycles: u16) -> bool {
        // The old Blargg test shell terminates with JMP to itself. Detect the
        // instruction's actual read trace, not an intermediate result byte.
        cycles == 3
            && self.reads == 3
            && self.values[0] == 0x4c
            && self.addresses[0] >= 0x8000
            && self.addresses[1] == self.addresses[0].wrapping_add(1)
            && self.addresses[2] == self.addresses[0].wrapping_add(2)
            && (self.values[1] as u16 | ((self.values[2] as u16) << 8)) == self.addresses[0]
            && self.inner.ppu.ctrl & 0x80 == 0
    }
}

fn run_rom(relative: &str, result_addr: u16) {
    let root = env::var("NES_TEST_ROMS").expect("Set NES_TEST_ROMS to the nes-test-roms checkout");
    let mut rom = Rom::load(Path::new(&root).join(relative)).unwrap();
    assert_eq!(
        rom.mapper, 0,
        "This test adapter supports only fixed NROM fixtures"
    );
    assert!(rom.prg_rom.len() == 1 || rom.prg_rom.len() == 2);
    assert!(rom.chr_rom.len() <= 1);
    let chr = ::std::mem::replace(&mut rom.chr_rom, Vec::new());
    // Test-only adapter: mapper 2's initial banks reproduce fixed 16/32 KiB
    // NROM mapping. Preload fixed CHR data; application mapper support stays
    // unchanged, and the CPU, DMA, PPU ports, and clock loop run normally.
    rom.mapper = 2;
    let mut inner = MemoryMappingInterconnect::new(rom);
    for (addr, value) in chr
        .iter()
        .flat_map(|bank| bank.iter())
        .take(8192)
        .enumerate()
    {
        inner.ppu.vram.write(addr as u16, *value);
    }
    let mut bus = RomBus {
        inner: inner,
        reads: 0,
        addresses: [0; 3],
        values: [0; 3],
    };
    let mut cpu = Cpu::new();
    cpu.reset(&mut bus);
    let mut elapsed = 0u64;
    while elapsed < 300 * 29781 {
        bus.reads = 0;
        let cycles = cpu.step(&mut bus);
        let halted = bus.halted(cycles);
        elapsed += cycles as u64;
        if halted {
            let result = bus.inner.read_word(result_addr);
            println!("{}: result={}, CPU cycles={}", relative, result, elapsed);
            assert_eq!(
                result, 1,
                "{} failed; see the upstream failure-code documentation",
                relative
            );
            return;
        }
    }
    panic!(
        "{} did not reach the test shell's terminal loop within 300 frames",
        relative
    );
}

macro_rules! public_rom {
    ($name:ident, $path:expr) => {
        public_rom!($name, $path, 0xf8);
    };
    ($name:ident, $path:expr, $result:expr) => {
        #[test]
        #[ignore = "Requires external NES_TEST_ROMS fixtures; see docs/sprite-rendering.md"]
        fn $name() {
            run_rom($path, $result);
        }
    };
}

public_rom!(hit_01_basics, "sprite_hit_tests_2005.10.05/01.basics.nes");
public_rom!(
    hit_02_alignment,
    "sprite_hit_tests_2005.10.05/02.alignment.nes"
);
public_rom!(hit_03_corners, "sprite_hit_tests_2005.10.05/03.corners.nes");
public_rom!(hit_04_flip, "sprite_hit_tests_2005.10.05/04.flip.nes");
public_rom!(
    hit_05_left_clip,
    "sprite_hit_tests_2005.10.05/05.left_clip.nes"
);
public_rom!(
    hit_06_right_edge,
    "sprite_hit_tests_2005.10.05/06.right_edge.nes"
);
public_rom!(
    hit_07_screen_bottom,
    "sprite_hit_tests_2005.10.05/07.screen_bottom.nes"
);
public_rom!(
    hit_08_double_height,
    "sprite_hit_tests_2005.10.05/08.double_height.nes"
);
public_rom!(
    hit_09_timing_basics,
    "sprite_hit_tests_2005.10.05/09.timing_basics.nes"
);
public_rom!(
    hit_10_timing_order,
    "sprite_hit_tests_2005.10.05/10.timing_order.nes"
);
public_rom!(
    hit_11_edge_timing,
    "sprite_hit_tests_2005.10.05/11.edge_timing.nes"
);
public_rom!(overflow_01_basics, "sprite_overflow_tests/1.Basics.nes");
public_rom!(overflow_02_details, "sprite_overflow_tests/2.Details.nes");
public_rom!(overflow_03_timing, "sprite_overflow_tests/3.Timing.nes");
public_rom!(overflow_04_obscure, "sprite_overflow_tests/4.Obscure.nes");
public_rom!(overflow_05_emulator, "sprite_overflow_tests/5.Emulator.nes");
public_rom!(oam_ram, "blargg_ppu_tests_2005.09.15b/sprite_ram.nes", 0xf0);
