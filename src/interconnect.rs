use rom::Rom;
use mapper::Mapper;
use mapper::unrom::Unrom;
use ppu::Ppu;

pub struct Interconnect {
    mapper: Box<Mapper>,
    ram: [u8; 2048],
    pub ppu: Ppu,
}

enum MappedAddress {
    Ram(usize),
    PpuControlRegister,
    PpuMaskRegister,
    PpuStatusRegister,
    SprRamAddressRegister,
    SprRamIoRegister,
    VramAddressRegister1,
    VramAddressRegister2,
    VramIoRegister,
    PrgRom,
}

fn map_addr(addr: u16) -> MappedAddress {
    match addr {
        0x0000...0x1fff => MappedAddress::Ram(addr as usize % 2048),
        0x8000...0xffff => MappedAddress::PrgRom,
        0x2000...0x3fff => {
            match (addr - 0x2000) % 8 {
                0 => MappedAddress::PpuControlRegister,
                1 => MappedAddress::PpuMaskRegister,
                2 => MappedAddress::PpuStatusRegister,
                3 => MappedAddress::SprRamAddressRegister,
                4 => MappedAddress::SprRamIoRegister,
                5 => MappedAddress::VramAddressRegister1,
                6 => MappedAddress::VramAddressRegister2,
                7 => MappedAddress::VramIoRegister,
                _ => unreachable!(),
            }
        }
        _ => panic!("Unmappable address: {:x}", addr),
    }
}

impl Interconnect {
    pub fn new(rom: Rom) -> Interconnect {
        let mapper = match rom.mapper {
            2 => Unrom::new(rom),
            _ => panic!("Unimplemented mapper"),
        };

        Interconnect {
            mapper: Box::new(mapper),
            ram: [0; 2048],
            ppu: Ppu::new(),
        }
    }

    pub fn read_double(&self, addr: u16) -> u16 {
        ((self.read_word(addr + 1) as u16) << 8) + self.read_word(addr) as u16
    }

    pub fn read_word(&self, addr: u16) -> u8 {
        match map_addr(addr) {
            MappedAddress::Ram(addr) => self.ram[addr],
            MappedAddress::PrgRom => self.mapper.read(addr),
            MappedAddress::PpuStatusRegister => self.ppu.status,
            _ => panic!("Reading from unimplemented memory address: {:x}", addr),
        }
    }

    pub fn write_word(&mut self, addr: u16, value: u8) {
        match map_addr(addr) {
            MappedAddress::Ram(addr) => self.ram[addr] = value,
            MappedAddress::PrgRom => self.mapper.write(addr, value),
            MappedAddress::PpuControlRegister => self.ppu.ctrl = value,
            MappedAddress::PpuMaskRegister => self.ppu.mask = value,
            _ => {
                println!("WARNING: Writing to unimplemented memory address: {:x}",
                         addr)
            }
        }
    }

    pub fn write_double(&mut self, addr: u16, value: u16) {
        match map_addr(addr) {
            MappedAddress::Ram(addr) => {
                self.ram[addr] = value as u8;
                self.ram[addr + 1] = (value >> 8) as u8;
            }
            _ => {
                println!("WARNING: Writing to unimplemented memory address: {:x}",
                         addr)
            }
        }
    }
}
