use rom::Rom;
use mapper::Mapper;
use mapper::unrom::Unrom;
use ppu::Ppu;

pub trait Interconnect {
    fn read_double(&mut self, addr: u16) -> u16;
    fn read_word(&mut self, addr: u16) -> u8;
    fn write_word(&mut self, addr: u16, value: u8);
    fn write_double(&mut self, addr: u16, value: u16);
}

pub struct MemoryMappingInterconnect {
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
    PapuPulse1ControlRegister,
    PapuPulse1RampControlRegister,
    PapuPulse1FineTuneRegister,
    PapuPulse1CoarseTuneRegister,
    PapuPulse2ControlRegister,
    PapuPulse2RampControlRegister,
    PapuPulse2FineTuneRegister,
    PapuPulse2CoarseTuneRegister,
    PapuTriangleControlRegister1,
    PapuTriangleControlRegister2,
    PapuTriangleFrequencyRegister1,
    PapuTriangleFrequencyRegister2,
    PapuNoiseControlRegister1,
    PapuNoiseFrequencyRegister1,
    PapuNoiseFrequencyRegister2,
    PapuDeltaModulationControlRegister,
    PapuDeltaModulationDaRegister,
    PapuDeltaModulationAddressRegister,
    PapuDeltaModulationDataLengthRegister,
    SpriteDmaRegister,
    PapuSoundVerticalClockSignalRegister,
    Joypad1,
    Joypad2,
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
        0x4000 => MappedAddress::PapuPulse1ControlRegister,
        0x4001 => MappedAddress::PapuPulse1RampControlRegister,
        0x4002 => MappedAddress::PapuPulse1FineTuneRegister,
        0x4003 => MappedAddress::PapuPulse1CoarseTuneRegister,
        0x4004 => MappedAddress::PapuPulse2ControlRegister,
        0x4005 => MappedAddress::PapuPulse2RampControlRegister,
        0x4006 => MappedAddress::PapuPulse2FineTuneRegister,
        0x4007 => MappedAddress::PapuPulse2CoarseTuneRegister,
        0x4008 => MappedAddress::PapuTriangleControlRegister1,
        0x4009 => MappedAddress::PapuTriangleControlRegister2,
        0x400A => MappedAddress::PapuTriangleFrequencyRegister1,
        0x400B => MappedAddress::PapuTriangleFrequencyRegister2,
        0x400C => MappedAddress::PapuNoiseControlRegister1,
        0x400E => MappedAddress::PapuNoiseFrequencyRegister1,
        0x400F => MappedAddress::PapuNoiseFrequencyRegister2,
        0x4010 => MappedAddress::PapuDeltaModulationControlRegister,
        0x4011 => MappedAddress::PapuDeltaModulationDaRegister,
        0x4012 => MappedAddress::PapuDeltaModulationAddressRegister,
        0x4013 => MappedAddress::PapuDeltaModulationDataLengthRegister,
        0x4014 => MappedAddress::SpriteDmaRegister,
        0x4015 => MappedAddress::PapuSoundVerticalClockSignalRegister,
        0x4016 => MappedAddress::Joypad1,
        0x4017 => MappedAddress::Joypad2,
        _ => panic!("Unmappable address: {:x}", addr),
    }
}

impl MemoryMappingInterconnect {
    pub fn new(rom: Rom) -> MemoryMappingInterconnect {
        let mapper = match rom.mapper {
            2 => Unrom::new(rom),
            _ => panic!("Unimplemented mapper"),
        };

        MemoryMappingInterconnect {
            mapper: Box::new(mapper),
            ram: [0; 2048],
            ppu: Ppu::new(),
        }
    }
}

impl Interconnect for MemoryMappingInterconnect {
    fn read_double(&mut self, addr: u16) -> u16 {
        ((self.read_word(addr + 1) as u16) << 8) + self.read_word(addr) as u16
    }

    fn read_word(&mut self, addr: u16) -> u8 {
        match map_addr(addr) {
            MappedAddress::Ram(addr) => self.ram[addr],
            MappedAddress::PrgRom => self.mapper.read(addr),
            MappedAddress::PpuStatusRegister => self.ppu.read_status(),
            MappedAddress::Joypad1 => 0,
            MappedAddress::Joypad2 => 0,
            _ => panic!("Reading from unimplemented memory address: {:x}", addr),
        }
    }

    fn write_word(&mut self, addr: u16, value: u8) {
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

    fn write_double(&mut self, addr: u16, value: u16) {
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
