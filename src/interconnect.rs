use rom::Rom;
use mapper::Mapper;
use mapper::unrom::Unrom;

pub struct Interconnect {
    mapper: Box<Mapper>,
}

impl Interconnect {
    pub fn new(rom: Rom) -> Interconnect {
        let mapper = match rom.mapper {
            2 => Unrom::new(rom),
            _ => panic!("Unimplemented mapper"),
        };

        Interconnect { mapper: Box::new(mapper) }
    }

    pub fn read_double(&self, addr: u16) -> u16 {
        ((self.read_word(addr + 1) as u16) << 8) + self.read_word(addr) as u16
    }

    pub fn read_word(&self, addr: u16) -> u8 {
        match addr {
            0x8000...0xffff => self.mapper.read(addr),
            _ => 0,
        }
    }
}
