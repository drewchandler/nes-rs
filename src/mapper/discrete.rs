use mapper::{Mapper, memory::Memory};
use rom::{Mirroring, Rom};

// Discrete-logic boards share memory storage, but retain their own register
// address decoding, data-bit wiring, bus conflicts, and nametable controls.
pub struct Discrete {
    id: u16,
    mem: Memory,
    bank: u8,
    outer: usize,
    chr_lo: usize,
    chr_hi: usize,
    mirror: Option<Mirroring>,
    nina001: bool,
    holy_diver: bool,
}
impl Discrete {
    pub fn new(rom: Rom) -> Discrete {
        let id = rom.mapper;
        let nina001 = id == 34 && (rom.submapper == 1 ||
            (rom.submapper == 0 && !rom.chr_rom.is_empty()));
        let holy_diver = id == 78 && (rom.submapper == 3 ||
            (!rom.nes2 && rom.mirroring == Mirroring::FourScreen));
        let mirror = match id {
            7 | 89 | 152 => Some(Mirroring::SingleLower),
            13 => Some(Mirroring::Vertical),
            78 => Some(if holy_diver { Mirroring::Horizontal } else { Mirroring::SingleLower }),
            71 if rom.submapper == 1 => Some(Mirroring::SingleLower),
            _ => None,
        };
        Discrete { id, mem: Memory::new(rom), bank: 0, outer: 0,
            chr_lo: 0, chr_hi: 0, mirror, nina001, holy_diver }
    }
    fn conflicts(&self) -> bool {
        match self.id {
            3 => self.mem.submapper != 1,
            7 | 11 => self.mem.submapper == 2,
            34 => !self.nina001,
            13 | 66 | 70 | 78 | 89 | 93 | 94 | 152 | 180 => true,
            _ => false,
        }
    }
    fn chr_addr(&self, addr: u16) -> usize {
        if self.id == 13 {
            return Memory::chr_addr(addr, 4096, if addr < 0x1000 { 0 } else { (self.bank & 3) as usize });
        }
        if self.id == 34 && self.nina001 {
            return Memory::chr_addr(addr, 4096, if addr < 0x1000 { self.chr_lo } else { self.chr_hi });
        }
        let bank = match self.id {
            3 | 66 => self.bank & 3,
            11 | 78 => self.bank >> 4,
            38 => (self.bank >> 2) & 3,
            70 | 140 | 152 => self.bank & 15,
            79 => self.bank & 7,
            87 => ((self.bank & 1) << 1) | ((self.bank >> 1) & 1),
            89 => (self.bank & 7) | ((self.bank & 0x80) >> 4),
            _ => 0,
        };
        Memory::chr_addr(addr, 8192, bank as usize)
    }
    fn set_mirror(&mut self, value: bool) {
        self.mirror = Some(if value { Mirroring::SingleUpper } else { Mirroring::SingleLower });
    }
}
impl Mapper for Discrete {
    fn read(&self, addr: u16) -> u8 {
        let bank32 = match self.id {
            7 => Some((self.bank & 7) as usize),
            11 | 38 => Some((self.bank & 3) as usize),
            34 => Some((self.bank & if self.nina001 { 1 } else { 3 }) as usize),
            66 | 140 => Some(((self.bank >> 4) & 3) as usize),
            79 => Some(((self.bank >> 3) & 1) as usize),
            3 | 13 | 87 => Some(0),
            _ => None,
        };
        if let Some(bank) = bank32 { return self.mem.prg(addr, 32768, bank); }
        let last = self.mem.prg.len() / 16384 - 1;
        let bank = match self.id {
            180 => if addr < 0xc000 { 0 } else { (self.bank & 7) as usize },
            232 => self.outer + if addr < 0xc000 { (self.bank & 3) as usize } else { 3 },
            _ if addr >= 0xc000 => last,
            70 | 89 | 93 | 152 => ((self.bank >> 4) & 7) as usize,
            71 => (self.bank & 15) as usize,
            78 => (self.bank & 7) as usize,
            94 => ((self.bank >> 2) & 7) as usize,
            _ => unreachable!(),
        };
        self.mem.prg(addr, 16384, bank)
    }
    fn write(&mut self, addr: u16, mut value: u8) {
        if self.conflicts() { value &= self.read(addr); }
        match self.id {
            34 if self.nina001 => return,
            38 | 79 | 87 | 140 => return, // These boards decode low CPU addresses.
            71 => {
                if addr >= 0xc000 { self.bank = value; }
                else if (self.mem.submapper == 1 && addr < 0xa000) ||
                    (!self.mem.nes2 && addr >= 0x9000 && addr < 0xa000) {
                    self.set_mirror(value & 0x10 != 0);
                }
                return;
            }
            232 => {
                if addr < 0xc000 { self.outer = ((value >> 3) & 3) as usize * 4; }
                else { self.bank = value; }
                return;
            }
            _ => self.bank = value,
        }
        match self.id {
            7 => self.set_mirror(value & 0x10 != 0),
            78 if self.holy_diver => self.mirror = Some(if value & 8 == 0 {
                Mirroring::Horizontal } else { Mirroring::Vertical }),
            78 | 89 => self.set_mirror(value & 8 != 0),
            152 => self.set_mirror(value & 0x80 != 0),
            _ => {}
        }
    }
    fn read_low(&self, addr: u16) -> Option<u8> {
        if self.nina001 || self.id == 3 { self.mem.read_ram(addr) } else { None }
    }
    fn write_low(&mut self, addr: u16, value: u8) {
        if self.nina001 {
            self.mem.write_ram(addr, value);
            match addr {
                0x7ffd => self.bank = value & 1,
                0x7ffe => self.chr_lo = (value & 15) as usize,
                0x7fff => self.chr_hi = (value & 15) as usize,
                _ => {}
            }
        } else {
            match self.id {
                3 => self.mem.write_ram(addr, value),
                38 if addr >= 0x7000 => self.bank = value,
                79 if addr & 0xe100 == 0x4100 => self.bank = value,
                87 | 140 if addr >= 0x6000 => self.bank = value,
                _ => {}
            }
        }
    }
    fn mirroring(&self) -> Option<Mirroring> { self.mirror }
    fn read_chr(&self, addr: u16) -> Option<u8> {
        if self.id == 93 && self.bank & 1 == 0 { None }
        else { Some(self.mem.chr.read(self.chr_addr(addr))) }
    }
    fn write_chr(&mut self, addr: u16, value: u8) {
        if self.id == 93 && self.bank & 1 == 0 { return; }
        let offset = self.chr_addr(addr); self.mem.chr.write(offset, value);
    }
}
