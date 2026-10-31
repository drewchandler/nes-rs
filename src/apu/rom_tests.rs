// External hardware-tested fixtures; see docs/audio.md for pinned sources.
use cpu::Cpu;
use interconnect::{Interconnect, MemoryMappingInterconnect};
use rom::Rom;
use std::env;
use std::path::Path;

struct TestBus {
    inner: MemoryMappingInterconnect,
    // Test-shell SRAM only. The application still supports mapper 2/CHR RAM.
    ram: [u8; 8192],
    reads: usize,
    addresses: [u16; 3],
    values: [u8; 3],
    nmi_enabled: bool,
}

impl TestBus {
    fn ram_address(addr: u16) -> Option<usize> {
        if addr >= 0x6000 && addr < 0x8000 {
            Some((addr - 0x6000) as usize)
        } else {
            None
        }
    }
    fn halted(&self, cycles: u16) -> bool {
        cycles == 3
            && self.reads == 3
            && self.values[0] == 0x4c
            && self.addresses[0] >= 0x8000
            && self.addresses[1] == self.addresses[0].wrapping_add(1)
            && self.addresses[2] == self.addresses[0].wrapping_add(2)
            && (self.values[1] as u16 | ((self.values[2] as u16) << 8)) == self.addresses[0]
            && !self.nmi_enabled
    }
    fn text(&self) -> String {
        let end = self.ram[4..].iter().position(|&b| b == 0).unwrap_or(8188);
        String::from_utf8_lossy(&self.ram[4..4 + end]).into_owned()
    }
}

impl Interconnect for TestBus {
    fn read_word(&mut self, addr: u16) -> u8 {
        match Self::ram_address(addr) {
            Some(offset) => self.ram[offset],
            None => self.inner.read_word(addr),
        }
    }
    fn write_word(&mut self, addr: u16, value: u8) {
        if let Some(offset) = Self::ram_address(addr) {
            self.ram[offset] = value;
        } else {
            self.inner.write_word(addr, value);
        }
    }
    fn read_double(&mut self, addr: u16) -> u16 {
        self.read_word(addr) as u16 | ((self.read_word(addr.wrapping_add(1)) as u16) << 8)
    }
    fn write_double(&mut self, addr: u16, value: u16) {
        self.write_word(addr, value as u8);
        self.write_word(addr.wrapping_add(1), (value >> 8) as u8);
    }
    fn cpu_read(&mut self, addr: u16) -> u8 {
        let mapped = self.inner.cpu_read(addr);
        let value = Self::ram_address(addr)
            .map(|offset| self.ram[offset])
            .unwrap_or(mapped);
        if self.reads < 3 {
            self.addresses[self.reads] = addr;
            self.values[self.reads] = value;
        }
        self.reads += 1;
        value
    }
    fn cpu_write(&mut self, addr: u16, value: u8) {
        self.inner.cpu_write(addr, value);
        if addr >= 0x2000 && addr < 0x4000 && addr & 7 == 0 {
            self.nmi_enabled = value & 0x80 != 0;
        }
        if let Some(offset) = Self::ram_address(addr) {
            self.ram[offset] = value;
        }
    }
    fn oam_dma_read(&mut self, addr: u16) -> u8 {
        self.inner.oam_dma_read(addr)
    }
    fn take_stall_cycles(&mut self) -> u16 {
        self.inner.take_stall_cycles()
    }
    fn poll_irq(&self) -> bool {
        self.inner.poll_irq()
    }
    fn take_nmi(&mut self) -> bool {
        self.inner.take_nmi()
    }
    fn reset_cpu(&mut self) {
        self.inner.reset_cpu();
    }
}

fn run_rom(relative: &str, legacy: bool) {
    let root = env::var("NES_TEST_ROMS").expect("Set NES_TEST_ROMS; see docs/audio.md");
    let rom = Rom::load(Path::new(&root).join(relative)).unwrap();
    assert_eq!(rom.mapper, 0, "Test adapter requires fixed NROM");
    assert!(rom.prg_rom.len() == 1 || rom.prg_rom.len() == 2);
    assert!(rom.chr_rom.len() <= 1);
    let inner = MemoryMappingInterconnect::new(rom);
    let mut bus = TestBus {
        inner,
        ram: [0; 8192],
        reads: 0,
        addresses: [0; 3],
        values: [0; 3],
        nmi_enabled: false,
    };
    let mut cpu = Cpu::new();
    cpu.reset(&mut bus);
    let mut reset_at = None;
    while bus.inner.cpu_cycles < 20_000_000 {
        bus.reads = 0;
        let cycles = cpu.step(&mut bus);
        if legacy {
            if bus.halted(cycles) {
                let result = bus.read_word(0xf0);
                println!(
                    "{}: result={}, cycles={}",
                    relative, result, bus.inner.cpu_cycles
                );
                assert_eq!(result, 1, "{} failed (legacy shell result $F0)", relative);
                return;
            }
        } else if bus.ram[1..4] == [0xde, 0xb0, 0x61] {
            match bus.ram[0] {
                0x80 => reset_at = None,
                0x81 => {
                    let deadline = *reset_at.get_or_insert(bus.inner.cpu_cycles + 180_000);
                    if bus.inner.cpu_cycles >= deadline {
                        cpu.reset(&mut bus);
                        reset_at = None;
                    }
                }
                result => {
                    println!(
                        "{}: result={}, cycles={}\n{}",
                        relative,
                        result,
                        bus.inner.cpu_cycles,
                        bus.text()
                    );
                    assert_eq!(result, 0, "{}: {}", relative, bus.text());
                    return;
                }
            }
        }
    }
    panic!("{} timed out: {}", relative, bus.text());
}

macro_rules! fixture {
    ($name:ident, $path:expr, $legacy:expr) => {
        #[test]
        #[ignore = "Requires external NES_TEST_ROMS fixtures; see docs/audio.md"]
        fn $name() {
            run_rom($path, $legacy);
        }
    };
}

fixture!(apu_01_length, "apu_test/rom_singles/1-len_ctr.nes", false);
fixture!(apu_02_table, "apu_test/rom_singles/2-len_table.nes", false);
fixture!(apu_03_irq, "apu_test/rom_singles/3-irq_flag.nes", false);
fixture!(apu_04_jitter, "apu_test/rom_singles/4-jitter.nes", false);
fixture!(
    apu_05_length_timing,
    "apu_test/rom_singles/5-len_timing.nes",
    false
);
fixture!(
    apu_06_irq_timing,
    "apu_test/rom_singles/6-irq_flag_timing.nes",
    false
);
fixture!(apu_07_dmc, "apu_test/rom_singles/7-dmc_basics.nes", false);
fixture!(
    apu_08_dmc_rates,
    "apu_test/rom_singles/8-dmc_rates.nes",
    false
);
fixture!(old_01_length, "blargg_apu_2005.07.30/01.len_ctr.nes", true);
fixture!(old_02_table, "blargg_apu_2005.07.30/02.len_table.nes", true);
fixture!(old_03_irq, "blargg_apu_2005.07.30/03.irq_flag.nes", true);
fixture!(
    old_04_jitter,
    "blargg_apu_2005.07.30/04.clock_jitter.nes",
    true
);
fixture!(
    old_05_length_mode0,
    "blargg_apu_2005.07.30/05.len_timing_mode0.nes",
    true
);
fixture!(
    old_06_length_mode1,
    "blargg_apu_2005.07.30/06.len_timing_mode1.nes",
    true
);
fixture!(
    old_07_irq_flag_timing,
    "blargg_apu_2005.07.30/07.irq_flag_timing.nes",
    true
);
fixture!(
    old_08_irq_entry,
    "blargg_apu_2005.07.30/08.irq_timing.nes",
    true
);
fixture!(
    old_09_reset,
    "blargg_apu_2005.07.30/09.reset_timing.nes",
    true
);
fixture!(
    old_10_halt,
    "blargg_apu_2005.07.30/10.len_halt_timing.nes",
    true
);
fixture!(
    old_11_reload,
    "blargg_apu_2005.07.30/11.len_reload_timing.nes",
    true
);
fixture!(reset_4015, "apu_reset/4015_cleared.nes", false);
fixture!(reset_4017_timing, "apu_reset/4017_timing.nes", false);
fixture!(reset_4017_written, "apu_reset/4017_written.nes", false);
fixture!(reset_irq, "apu_reset/irq_flag_cleared.nes", false);
fixture!(reset_length, "apu_reset/len_ctrs_enabled.nes", false);
fixture!(reset_immediate, "apu_reset/works_immediately.nes", false);
