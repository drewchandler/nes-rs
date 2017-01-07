use rom::Rom;

pub struct Interconnect {
    rom: Rom,
}

impl Interconnect {
    pub fn new(rom: Rom) -> Interconnect {
        Interconnect { rom: rom }
    }

    pub fn read_double(&self, addr: u16) -> u16 {
        ((self.read_word(addr + 1) as u16) << 8) + self.read_word(addr) as u16
    }

    pub fn read_word(&self, addr: u16) -> u8 {
        0
    }
}
