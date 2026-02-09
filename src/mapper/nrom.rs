use mapper::Mapper;

pub struct Nrom {
    prg_rom: Vec<Vec<u8>>,
}

impl Nrom {
    pub fn new(prg_rom: Vec<Vec<u8>>) -> Nrom {
        Nrom { prg_rom }
    }
}

impl Mapper for Nrom {
    fn read(&self, addr: u16) -> u8 {
        match addr {
            0x8000..=0xffff => {
                if self.prg_rom.len() == 1 {
                    let offset = (addr - 0x8000) as usize % 0x4000;
                    self.prg_rom[0][offset]
                } else {
                    let bank = ((addr - 0x8000) as usize) / 0x4000;
                    let offset = (addr - 0x8000) as usize % 0x4000;
                    self.prg_rom[bank][offset]
                }
            }
            _ => panic!("Illegal memory address for mapper: {}", addr),
        }
    }

    fn write(&mut self, addr: u16, _value: u8) {
        match addr {
            0x8000..=0xffff => {}
            _ => panic!("NROM unimplemented write {:x}", addr),
        }
    }
}
