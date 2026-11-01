use mapper::chr::ChrMemory;
use mapper::Mapper;
use rom::Rom;

pub struct Nrom {
    prg_rom: Vec<Vec<u8>>,
    chr: ChrMemory,
}

impl Nrom {
    pub fn new(rom: Rom) -> Nrom {
        assert!(rom.prg_rom.len() == 1 || rom.prg_rom.len() == 2,
                "NROM requires 16 or 32 KiB PRG ROM");
        Nrom {
            prg_rom: rom.prg_rom,
            chr: ChrMemory::new(rom.chr_rom),
        }
    }
}

impl Mapper for Nrom {
    fn read(&self, addr: u16) -> u8 {
        assert!(addr >= 0x8000, "Invalid NROM CPU address");
        let offset = (addr - 0x8000) as usize;
        // NROM-128 repeats its 16 KiB bank; NROM-256 maps both banks.
        self.prg_rom[offset / 16384 % self.prg_rom.len()][offset % 16384]
    }

    fn write(&mut self, _addr: u16, _value: u8) {
        // NROM has no bank registers and CPU writes cannot change PRG ROM.
    }

    fn read_chr(&self, addr: u16) -> Option<u8> { Some(self.chr.read(addr as usize)) }
    fn write_chr(&mut self, addr: u16, value: u8) { self.chr.write(addr as usize, value); }
}

#[cfg(test)]
mod tests;
