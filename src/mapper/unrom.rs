use rom::Rom;
use mapper::Mapper;
use mapper::chr::ChrMemory;

pub struct Unrom {
    rom: Rom,
    active_bank: usize,
    chr: ChrMemory,
}

impl Unrom {
    pub fn new(rom: Rom) -> Unrom {
        assert!(rom.chr_rom.is_empty(), "UxROM requires CHR RAM");
        Unrom {
            rom: rom,
            active_bank: 0,
            chr: ChrMemory::new(Vec::new()),
        }
    }
}

impl Mapper for Unrom {
    fn read_chr(&self, addr: u16) -> Option<u8> { Some(self.chr.read(addr as usize)) }
    fn write_chr(&mut self, addr: u16, value: u8) { self.chr.write(addr as usize, value); }

    fn read(&self, addr: u16) -> u8 {
        let prg_rom = &self.rom.prg_rom;

        match addr {
            0x8000...0xbfff => prg_rom[self.active_bank][(addr - 0x8000) as usize],
            0xc000...0xffff => prg_rom[prg_rom.len() - 1][(addr - 0xc000) as usize],
            _ => panic!("Illegal memory address for mapper: {}", addr),
        }
    }

    fn write(&mut self, addr: u16, value: u8) {
        match addr {
            0x8000...0xffff => {
                // NES 2.0 mapper 2: submapper 1 has no bus conflicts; 2
                // ANDs the write with the ROM byte driven at that address.
                // Keep the existing behavior for unspecified iNES boards.
                let value = if self.rom.submapper == 2 { value & self.read(addr) } else { value };
                self.active_bank = value as usize % self.rom.prg_rom.len();
            },
            _ => panic!("UNROM unimplemented write {:x} = {}", addr, value),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn nes2_submapper_selects_bus_conflict_behavior() {
        for submapper in 0..=2 {
            let mut bytes = vec![0; 16 + 4 * 16384];
            bytes[..4].copy_from_slice(b"NES\x1a");
            bytes[4] = 4;
            bytes[6] = 0x20;
            bytes[7] = 0x08;
            bytes[8] = submapper << 4;
            bytes[11] = 7;
            for bank in 0..4 {
                for byte in &mut bytes[16 + bank * 16384..16 + (bank + 1) * 16384] {
                    *byte = bank as u8 + 1;
                }
            }
            let mut mapper = Unrom::new(Rom::read(&mut Cursor::new(bytes)).unwrap());
            assert_eq!(mapper.read(0x8000), 1);
            mapper.write(0x8000, 3);
            // With conflicts, 3 AND the old bank's ROM value (1) selects bank 1.
            assert_eq!(mapper.read(0x8000), if submapper == 2 { 2 } else { 4 });
            assert_eq!(mapper.read(0xc000), 4, "last bank stays fixed");
        }
    }
}
