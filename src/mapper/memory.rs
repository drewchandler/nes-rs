use mapper::chr::ChrMemory;
use rom::{Mirroring, Rom};

pub struct Memory {
    pub prg: Vec<u8>,
    pub chr: ChrMemory,
    pub ram: Vec<u8>,
    pub mirroring: Mirroring,
    pub submapper: u8,
    pub nes2: bool,
}

impl Memory {
    pub fn new(rom: Rom) -> Memory {
        Memory {
            prg: rom.prg_rom.into_iter().flat_map(|b| b.into_iter()).collect(),
            chr: ChrMemory::with_ram(rom.chr_rom, rom.chr_ram_size),
            ram: vec![0; rom.prg_ram_size],
            mirroring: rom.mirroring,
            submapper: rom.submapper,
            nes2: rom.nes2,
        }
    }

    pub fn prg(&self, addr: u16, size: usize, bank: usize) -> u8 {
        self.prg[(bank * size + addr as usize % size) % self.prg.len()]
    }
    pub fn chr_addr(addr: u16, size: usize, bank: usize) -> usize {
        bank * size + addr as usize % size
    }
    pub fn read_ram(&self, addr: u16) -> Option<u8> {
        if addr < 0x6000 || addr > 0x7fff || self.ram.is_empty() { None }
        else { Some(self.ram[(addr - 0x6000) as usize % self.ram.len()]) }
    }
    pub fn write_ram(&mut self, addr: u16, value: u8) {
        if addr >= 0x6000 && addr <= 0x7fff && !self.ram.is_empty() {
            let index = (addr - 0x6000) as usize % self.ram.len();
            self.ram[index] = value;
        }
    }
}
