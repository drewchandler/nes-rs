use rom::Rom;
use mapper::Mapper;

pub struct Unrom {
    rom: Rom,
    active_bank: usize,
}

impl Unrom {
    pub fn new(rom: Rom) -> Unrom {
        Unrom {
            rom: rom,
            active_bank: 0,
        }
    }
}

impl Mapper for Unrom {
    fn read(&self, addr: u16) -> u8 {
        let prg_rom = &self.rom.prg_rom;

        match addr {
            0x8000...0xbfff => prg_rom[self.active_bank][(addr - 0x8000) as usize],
            0xc000...0xffff => prg_rom[prg_rom.len() - 1][(addr - 0xc000) as usize],
            _ => panic!("Illegal memory address for mapper: {}", addr),
        }
    }
    fn write(&mut self, addr: u16, value: u8) {}
}
