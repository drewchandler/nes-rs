use mapper::Mapper;

pub struct Unrom {
    prg_rom: Vec<Vec<u8>>,
    active_bank: usize,
}

impl Unrom {
    pub fn new(prg_rom: Vec<Vec<u8>>) -> Unrom {
        Unrom {
            prg_rom: prg_rom,
            active_bank: 0,
        }
    }
}

impl Mapper for Unrom {
    fn read(&self, addr: u16) -> u8 {
        match addr {
            0x8000...0xbfff => self.prg_rom[self.active_bank][(addr - 0x8000) as usize],
            0xc000...0xffff => {
                self.prg_rom[self.prg_rom.len() - 1][(addr - 0xc000) as usize]
            }
            _ => panic!("Illegal memory address for mapper: {}", addr),
        }
    }

    fn write(&mut self, addr: u16, value: u8) {
        match addr {
            0x8000...0xffff => self.active_bank = value as usize % self.prg_rom.len(),
            _ => panic!("UNROM unimplemented write {:x} = {}", addr, value),
        }
    }
}
