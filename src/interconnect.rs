use joypad::Joypad;
use mapper::Mapper;
use mapper::unrom::Unrom;
use ppu::Ppu;
use rom::Rom;

pub trait Interconnect {
    fn read_double(&mut self, addr: u16) -> u16;
    fn read_word(&mut self, addr: u16) -> u8;
    fn write_word(&mut self, addr: u16, value: u8);
    fn write_double(&mut self, addr: u16, value: u16);
}

pub struct MemoryMappingInterconnect {
    mapper: Box<dyn Mapper>,
    ram: [u8; 2048],
    pub ppu: Ppu,
    pub joypad1: Joypad,
}

enum MappedAddress {
    Ram(usize),
    PpuControlRegister,
    PpuMaskRegister,
    PpuStatusRegister,
    SprRamAddressRegister,
    SprRamIoRegister,
    PpuScrollRegister,
    VramAddressRegister,
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
        0x0000..=0x1fff => MappedAddress::Ram(addr as usize % 2048),
        0x8000..=0xffff => MappedAddress::PrgRom,
        0x2000..=0x3fff => {
            match (addr - 0x2000) % 8 {
                0 => MappedAddress::PpuControlRegister,
                1 => MappedAddress::PpuMaskRegister,
                2 => MappedAddress::PpuStatusRegister,
                3 => MappedAddress::SprRamAddressRegister,
                4 => MappedAddress::SprRamIoRegister,
                5 => MappedAddress::PpuScrollRegister,
                6 => MappedAddress::VramAddressRegister,
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
        let Rom {
            prg_rom,
            chr_rom,
            mapper,
            mirroring,
            chr_ram_size,
        } = rom;

        let mapper = match mapper {
            2 => Unrom::new(prg_rom),
            _ => panic!("Unimplemented mapper"),
        };

        MemoryMappingInterconnect {
            mapper: Box::new(mapper),
            ram: [0; 2048],
            ppu: Ppu::new(chr_rom, chr_ram_size, mirroring),
            joypad1: Joypad::new(),
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
            MappedAddress::SprRamIoRegister => self.ppu.read_spr_ram_data(),
            MappedAddress::VramIoRegister => self.ppu.read_vram_data(),
            MappedAddress::Joypad1 => self.joypad1.read(),
            MappedAddress::Joypad2 => 0,
            _ => panic!("Reading from unimplemented memory address: {:x}", addr),
        }
    }

    fn write_word(&mut self, addr: u16, value: u8) {
        match map_addr(addr) {
            MappedAddress::Ram(addr) => self.ram[addr] = value,
            MappedAddress::PrgRom => self.mapper.write(addr, value),
            MappedAddress::PpuControlRegister => self.ppu.write_ctrl(value),
            MappedAddress::PpuMaskRegister => self.ppu.write_mask(value),
            MappedAddress::SprRamAddressRegister => self.ppu.write_spr_ram_addr(value),
            MappedAddress::SprRamIoRegister => self.ppu.write_spr_ram_data(value),
            MappedAddress::PpuScrollRegister => self.ppu.write_scroll(value),
            MappedAddress::Joypad1 => self.joypad1.strobe(),
            MappedAddress::VramAddressRegister => self.ppu.write_vram_addr(value),
            MappedAddress::VramIoRegister => self.ppu.write_vram_data(value),
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
