use mapper::{Mapper, memory::Memory};
use rom::{Mirroring, Rom};

// Irem G-101 (32) and Konami VRC1 (75) have independent PRG/CHR registers.
pub struct Registers {
    id: u16,
    mem: Memory,
    prg: [u8; 3],
    chr: [u8; 8],
    mode: bool,
    mirror: Mirroring,
}
impl Registers {
    pub fn new(rom: Rom) -> Registers {
        let id = rom.mapper;
        let mirror = if id == 32 && rom.submapper == 1 { Mirroring::SingleUpper } else { rom.mirroring };
        Registers { id, mem: Memory::new(rom), prg: [0, 1, 2], chr: [0; 8], mode: false, mirror }
    }
    fn chr_addr(&self, addr: u16) -> usize {
        let size = if self.id == 32 { 1024 } else { 4096 };
        Memory::chr_addr(addr, size, self.chr[addr as usize / size] as usize)
    }
}
impl Mapper for Registers {
    fn read(&self, addr: u16) -> u8 {
        let slot = (addr - 0x8000) as usize / 8192;
        let last = self.mem.prg.len() / 8192 - 1;
        let bank = if self.id == 75 {
            if slot < 3 { self.prg[slot] as usize } else { last }
        } else {
            match slot {
                0 => if self.mode { last - 1 } else { self.prg[0] as usize },
                1 => self.prg[1] as usize,
                2 => if self.mode { self.prg[0] as usize } else { last - 1 },
                _ => last,
            }
        };
        self.mem.prg(addr, 8192, bank)
    }
    fn write(&mut self, addr: u16, value: u8) {
        if self.id == 32 {
            match addr & 0xf000 {
                0x8000 => self.prg[0] = value & 31,
                0x9000 if self.mem.submapper == 0 => {
                    self.mode = value & 2 != 0;
                    self.mirror = if value & 1 == 0 { Mirroring::Vertical } else { Mirroring::Horizontal };
                }
                0xa000 => self.prg[1] = value & 31,
                0xb000 => self.chr[(addr & 7) as usize] = value,
                _ => {}
            }
        } else {
            match addr & 0xf000 {
                0x8000 => self.prg[0] = value & 15,
                0x9000 => {
                    self.mirror = if value & 1 == 0 { Mirroring::Vertical } else { Mirroring::Horizontal };
                    self.chr[0] = (self.chr[0] & 15) | ((value & 2) << 3);
                    self.chr[1] = (self.chr[1] & 15) | ((value & 4) << 2);
                }
                0xa000 => self.prg[1] = value & 15,
                0xc000 => self.prg[2] = value & 15,
                0xe000 => self.chr[0] = (self.chr[0] & 16) | (value & 15),
                0xf000 => self.chr[1] = (self.chr[1] & 16) | (value & 15),
                _ => {}
            }
        }
    }
    fn read_low(&self, addr: u16) -> Option<u8> {
        if self.id == 32 { self.mem.read_ram(addr) } else { None }
    }
    fn write_low(&mut self, addr: u16, value: u8) {
        if self.id == 32 { self.mem.write_ram(addr, value); }
    }
    fn mirroring(&self) -> Option<Mirroring> { Some(self.mirror) }
    fn read_chr(&self, addr: u16) -> Option<u8> { Some(self.mem.chr.read(self.chr_addr(addr))) }
    fn write_chr(&mut self, addr: u16, value: u8) {
        let offset = self.chr_addr(addr); self.mem.chr.write(offset, value);
    }
}
