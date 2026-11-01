use std::cell::Cell;
use mapper::{Mapper, memory::Memory};
use rom::{Mirroring, Rom};

// MMC2 (9) and MMC4 (10): pattern fetches latch the next 4 KiB CHR bank.
pub struct Mmc2 {
    mem: Memory,
    mmc4: bool,
    prg: u8,
    chr: [u8; 4],
    latch: [Cell<bool>; 2],
    mirroring: Mirroring,
}
impl Mmc2 {
    pub fn new(rom: Rom) -> Mmc2 {
        let mmc4 = rom.mapper == 10;
        let mirroring = rom.mirroring;
        Mmc2 { mem: Memory::new(rom), mmc4, prg: 0, chr: [0; 4],
            latch: [Cell::new(true), Cell::new(true)], mirroring }
    }
}
impl Mapper for Mmc2 {
    fn memory(&self) -> Option<&Memory> { Some(&self.mem) }
    fn memory_mut(&mut self) -> Option<&mut Memory> { Some(&mut self.mem) }
    fn read(&self, addr: u16) -> u8 {
        let size = if self.mmc4 { 16384 } else { 8192 };
        let slot = (addr - 0x8000) as usize / size;
        let bank = if slot == 0 { self.prg as usize }
            else { self.mem.prg.len() / size - (32768 / size - slot) };
        self.mem.prg(addr, size, bank)
    }
    fn write(&mut self, addr: u16, value: u8) {
        match addr & 0xf000 {
            0xa000 => self.prg = value & 15,
            0xb000 => self.chr[0] = value & 31,
            0xc000 => self.chr[1] = value & 31,
            0xd000 => self.chr[2] = value & 31,
            0xe000 => self.chr[3] = value & 31,
            0xf000 => self.mirroring = if value & 1 == 0 { Mirroring::Vertical } else { Mirroring::Horizontal },
            _ => {}
        }
    }
    fn read_low(&self, addr: u16) -> Option<u8> { self.mem.read_ram(addr) }
    fn write_low(&mut self, addr: u16, value: u8) { self.mem.write_ram(addr, value); }
    fn mirroring(&self) -> Option<Mirroring> { Some(self.mirroring) }
    fn read_chr(&self, addr: u16) -> Option<u8> {
        let half = (addr / 4096) as usize;
        let bank = self.chr[half * 2 + self.latch[half].get() as usize] as usize;
        let value = self.mem.chr.read(Memory::chr_addr(addr, 4096, bank));
        // The triggering read uses the previous bank; subsequent reads switch.
        let trigger = if self.mmc4 || half == 1 { addr & 0x0ff8 } else { addr & 0x0fff };
        if trigger == 0x0fd8 { self.latch[half].set(false); }
        else if trigger == 0x0fe8 { self.latch[half].set(true); }
        Some(value)
    }
    fn write_chr(&mut self, _addr: u16, _value: u8) {}
}

state_fields!(Mmc2, mem,prg,chr,latch,mirroring);
