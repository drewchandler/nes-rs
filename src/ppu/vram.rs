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
            0x0000..=0x1fff => self.chr[addr as usize],
            0x2000..=0x3eff => {
                let idx = self.map_nametable_addr(addr);
                self.nametable[idx]
            }
            0x3f00..=0x3fff => {
                let idx = Self::map_palette_addr(addr);
                self.palette[idx]
            }
            _ => 0,
        }
    }

    pub fn write(&mut self, addr: u16, value: u8) {
        let addr = self.normalize_addr(addr);
        match addr {
            0x0000..=0x1fff => {
                if self.chr_is_ram {
                    self.chr[addr as usize] = value;
                }
            }
            0x2000..=0x3eff => {
                let idx = self.map_nametable_addr(addr);
                self.nametable[idx] = value;
            }
            0x3f00..=0x3fff => {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn new_chr_ram(mirroring: Mirroring) -> Vram {
        Vram::new(Vec::new(), 0x2000, mirroring)
    }

    #[test]
    fn test_chr_ram_writeable() {
        let mut vram = new_chr_ram(Mirroring::Horizontal);
        vram.write(0x0000, 0xaa);
        assert_eq!(vram.read(0x0000), 0xaa);
    }

    #[test]
    fn test_chr_rom_readonly() {
        let mut vram = Vram::new(vec![0x55; 0x2000], 0, Mirroring::Horizontal);
        vram.write(0x0000, 0xaa);
        assert_eq!(vram.read(0x0000), 0x55);
    }

    #[test]
    fn test_palette_mirroring() {
        let mut vram = new_chr_ram(Mirroring::Horizontal);
        vram.write(0x3f10, 0x12);
        assert_eq!(vram.read(0x3f00), 0x12);
        vram.write(0x3f14, 0x34);
        assert_eq!(vram.read(0x3f04), 0x34);
    }

    #[test]
    fn test_nametable_mirroring_vertical() {
        let mut vram = new_chr_ram(Mirroring::Vertical);
        vram.write(0x2000, 0x01);
        vram.write(0x2400, 0x02);
        assert_eq!(vram.read(0x2800), 0x01);
        assert_eq!(vram.read(0x2c00), 0x02);
    }

    #[test]
    fn test_nametable_mirroring_horizontal() {
        let mut vram = new_chr_ram(Mirroring::Horizontal);
        vram.write(0x2000, 0x01);
        vram.write(0x2800, 0x02);
        assert_eq!(vram.read(0x2400), 0x01);
        assert_eq!(vram.read(0x2c00), 0x02);
    }

    #[test]
    fn test_vram_mirror_3000_range() {
        let mut vram = new_chr_ram(Mirroring::Horizontal);
        vram.write(0x3000, 0x7f);
        assert_eq!(vram.read(0x2000), 0x7f);
        vram.write(0x3eff, 0x55);
        assert_eq!(vram.read(0x2eff), 0x55);
    }
}
