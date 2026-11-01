use mapper::{Mapper, memory::Memory};
use rom::{Mirroring, Rom};

// MMC1B, including SUROM/SOROM/SXROM outer PRG and RAM banking.
pub struct Mmc1 {
    mem: Memory,
    shift: u8,
    control: u8,
    chr0: u8,
    chr1: u8,
    prg: u8,
    last_write: Option<u64>,
}
impl Mmc1 {
    pub fn new(rom: Rom) -> Mmc1 {
        Mmc1 { mem: Memory::new(rom), shift: 0x10, control: 0x0c,
            chr0: 0, chr1: 0, prg: 0, last_write: None }
    }
    fn ram_enabled(&self) -> bool {
        self.prg & 0x10 == 0 && !(self.mem.chr.is_ram() && self.mem.prg.len() <= 262144 &&
            self.mem.ram.len() == 8192 && self.chr0 & 0x10 != 0)
    }
    fn ram_offset(&self, addr: u16) -> usize {
        let bank = if self.mem.ram.len() >= 32768 { (self.chr0 >> 2) & 3 }
            else if self.mem.ram.len() >= 16384 { (self.chr0 >> 3) & 1 } else { 0 };
        (bank as usize * 8192 + (addr - 0x6000) as usize) % self.mem.ram.len()
    }
    fn chr_addr(&self, addr: u16) -> usize {
        let bank = if self.control & 0x10 == 0 {
            (self.chr0 & 0x1e) as usize + (addr / 4096) as usize
        } else if addr < 0x1000 { self.chr0 as usize } else { self.chr1 as usize };
        Memory::chr_addr(addr, 4096, bank)
    }
}
impl Mapper for Mmc1 {
    fn memory(&self) -> Option<&Memory> { Some(&self.mem) }
    fn memory_mut(&mut self) -> Option<&mut Memory> { Some(&mut self.mem) }
    fn read(&self, addr: u16) -> u8 {
        if self.mem.submapper == 5 { return self.mem.prg(addr, 32768, 0); }
        let outer = if self.mem.prg.len() > 262144 { (self.chr0 & 0x10) as usize } else { 0 };
        let bank = match (self.control >> 2) & 3 {
            0 | 1 => (self.prg & 0x0e) as usize + (addr >= 0xc000) as usize,
            2 => if addr < 0xc000 { 0 } else { (self.prg & 15) as usize },
            _ => if addr < 0xc000 { (self.prg & 15) as usize } else { (self.mem.prg.len() / 16384).min(16) - 1 },
        };
        self.mem.prg(addr, 16384, outer + bank)
    }
    fn write(&mut self, addr: u16, value: u8) {
        if value & 0x80 != 0 { self.shift = 0x10; self.control |= 0x0c; return; }
        let complete = self.shift & 1 != 0;
        self.shift = (self.shift >> 1) | ((value & 1) << 4);
        if complete {
            match (addr >> 13) & 3 {
                0 => self.control = self.shift,
                1 => self.chr0 = self.shift,
                2 => self.chr1 = self.shift,
                _ => self.prg = self.shift,
            }
            self.shift = 0x10;
        }
    }
    fn write_cpu(&mut self, addr: u16, value: u8, cycle: u64) {
        let consecutive = self.last_write.map_or(false, |last| cycle == last.wrapping_add(1));
        self.last_write = Some(cycle);
        // Only serial D0 is filtered; a D7 reset is accepted even on the
        // second write of a read-modify-write instruction.
        if value & 0x80 != 0 || !consecutive { self.write(addr, value); }
    }
    fn read_low(&self, addr: u16) -> Option<u8> {
        if addr >= 0x6000 && addr <= 0x7fff && !self.mem.ram.is_empty() && self.ram_enabled() {
            Some(self.mem.ram[self.ram_offset(addr)])
        } else { None }
    }
    fn write_low(&mut self, addr: u16, value: u8) {
        if addr >= 0x6000 && addr <= 0x7fff && !self.mem.ram.is_empty() && self.ram_enabled() {
            let index = self.ram_offset(addr); self.mem.ram[index] = value;
        }
    }
    fn mirroring(&self) -> Option<Mirroring> {
        Some(match self.control & 3 {
            0 => Mirroring::SingleLower, 1 => Mirroring::SingleUpper,
            2 => Mirroring::Vertical, _ => Mirroring::Horizontal,
        })
    }
    fn read_chr(&self, addr: u16) -> Option<u8> { Some(self.mem.chr.read(self.chr_addr(addr))) }
    fn write_chr(&mut self, addr: u16, value: u8) {
        let offset = self.chr_addr(addr); self.mem.chr.write(offset, value);
    }
}

state_fields!(Mmc1, mem,shift,control,chr0,chr1,prg,last_write;|s: &Mmc1| s.shift<32 && s.control<32 && s.chr0<32 && s.chr1<32 && s.prg<32);
