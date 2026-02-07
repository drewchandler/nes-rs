use rom::Mirroring;

pub struct Vram {
    chr: Vec<u8>,
    chr_is_ram: bool,
    nametable: Vec<u8>,
    palette: [u8; 32],
    mirroring: Mirroring,
}

impl Vram {
    pub fn new(chr_rom: Vec<u8>, chr_ram_size: usize, mirroring: Mirroring) -> Vram {
        let (chr, chr_is_ram) = if chr_rom.is_empty() && chr_ram_size > 0 {
            (vec![0u8; chr_ram_size], true)
        } else {
            (chr_rom, false)
        };

        let nametable_size = match mirroring {
            Mirroring::FourScreen => 0x1000,
            _ => 0x800,
        };

        Vram {
            chr: chr,
            chr_is_ram: chr_is_ram,
            nametable: vec![0u8; nametable_size],
            palette: [0u8; 32],
            mirroring: mirroring,
        }
    }

    pub fn read(&self, addr: u16) -> u8 {
        let addr = self.normalize_addr(addr);
        match addr {
            0x0000...0x1fff => self.chr[addr as usize],
            0x2000...0x3eff => {
                let idx = self.map_nametable_addr(addr);
                self.nametable[idx]
            }
            0x3f00...0x3fff => {
                let idx = Self::map_palette_addr(addr);
                self.palette[idx]
            }
            _ => 0,
        }
    }

    pub fn write(&mut self, addr: u16, value: u8) {
        let addr = self.normalize_addr(addr);
        match addr {
            0x0000...0x1fff => {
                if self.chr_is_ram {
                    self.chr[addr as usize] = value;
                }
            }
            0x2000...0x3eff => {
                let idx = self.map_nametable_addr(addr);
                self.nametable[idx] = value;
            }
            0x3f00...0x3fff => {
                let idx = Self::map_palette_addr(addr);
                self.palette[idx] = value;
            }
            _ => {}
        }
    }

    fn normalize_addr(&self, addr: u16) -> u16 {
        let addr = addr % 0x4000;
        if addr >= 0x3000 && addr <= 0x3eff {
            addr - 0x1000
        } else {
            addr
        }
    }

    fn map_nametable_addr(&self, addr: u16) -> usize {
        let addr = (addr - 0x2000) % 0x1000;
        match self.mirroring {
            Mirroring::Vertical => (addr % 0x800) as usize,
            Mirroring::Horizontal => {
                let table = addr / 0x400;
                let offset = addr % 0x400;
                match table {
                    0 | 1 => offset as usize,
                    _ => (0x400 + offset) as usize,
                }
            }
            Mirroring::FourScreen => addr as usize,
        }
    }

    fn map_palette_addr(addr: u16) -> usize {
        let mut idx = ((addr - 0x3f00) % 32) as usize;
        if idx >= 0x10 && idx % 4 == 0 {
            idx -= 0x10;
        }
        idx
    }
}
