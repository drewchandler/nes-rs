use joypad::Joypad;
use mapper::nrom::Nrom;
use mapper::unrom::Unrom;
use mapper::Mapper;
use ppu::Ppu;
use rom::Rom;
use std::env;

pub trait Interconnect {
    fn read_double(&mut self, addr: u16) -> u16;
    fn read_word(&mut self, addr: u16) -> u8;
    fn write_word(&mut self, addr: u16, value: u8);
    fn write_double(&mut self, addr: u16, value: u16);
    fn trace_cpu(&mut self, _pc: u16, _opcode: u8) {}
}

pub struct MemoryMappingInterconnect {
    mapper: Box<dyn Mapper>,
    ram: [u8; 2048],
    prg_ram: [u8; 0x2000],
    pub ppu: Ppu,
    pub joypad1: Joypad,
    last_cpu_pc: u16,
    last_cpu_opcode: u8,
    last_bus_value: u8,
    trace_expansion: bool,
}

enum MappedAddress {
    Ram(usize),
    PrgRam(usize),
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
    PapuNoiseUnusedRegister,
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
    ApuTestRegister,
    PrgRom,
}

fn map_addr(addr: u16) -> MappedAddress {
    match addr {
        0x0000..=0x1fff => MappedAddress::Ram(addr as usize % 2048),
        0x6000..=0x7fff => MappedAddress::PrgRam((addr - 0x6000) as usize),
        0x8000..=0xffff => MappedAddress::PrgRom,
        0x2000..=0x3fff => match (addr - 0x2000) % 8 {
            0 => MappedAddress::PpuControlRegister,
            1 => MappedAddress::PpuMaskRegister,
            2 => MappedAddress::PpuStatusRegister,
            3 => MappedAddress::SprRamAddressRegister,
            4 => MappedAddress::SprRamIoRegister,
            5 => MappedAddress::PpuScrollRegister,
            6 => MappedAddress::VramAddressRegister,
            7 => MappedAddress::VramIoRegister,
            _ => unreachable!(),
        },
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
        0x400D => MappedAddress::PapuNoiseUnusedRegister,
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
        0x4018..=0x401f => MappedAddress::ApuTestRegister,
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

        let mapper: Box<dyn Mapper> = match mapper {
            0 => Box::new(Nrom::new(prg_rom)),
            2 => Box::new(Unrom::new(prg_rom)),
            _ => panic!("Unimplemented mapper"),
        };

        MemoryMappingInterconnect {
            mapper,
            ram: [0; 2048],
            prg_ram: [0; 0x2000],
            ppu: Ppu::new(chr_rom, chr_ram_size, mirroring),
            joypad1: Joypad::new(),
            last_cpu_pc: 0,
            last_cpu_opcode: 0,
            last_bus_value: 0,
            trace_expansion: env::var("NES_TRACE_EXPANSION").is_ok(),
        }
    }

    fn is_expansion(addr: u16) -> bool {
        (0x4020..=0x5fff).contains(&addr)
    }
}

impl Interconnect for MemoryMappingInterconnect {
    fn read_double(&mut self, addr: u16) -> u16 {
        ((self.read_word(addr + 1) as u16) << 8) + self.read_word(addr) as u16
    }

    fn read_word(&mut self, addr: u16) -> u8 {
        if MemoryMappingInterconnect::is_expansion(addr) {
            if self.trace_expansion {
                println!(
                    "Expansion read {:04x} (pc={:04x} opcode={:02x}) bus={:02x}",
                    addr, self.last_cpu_pc, self.last_cpu_opcode, self.last_bus_value
                );
            }
            return self.last_bus_value;
        }

        let value = match map_addr(addr) {
            MappedAddress::Ram(addr) => self.ram[addr],
            MappedAddress::PrgRam(addr) => self.prg_ram[addr],
            MappedAddress::PrgRom => self.mapper.read(addr),
            MappedAddress::PpuStatusRegister => self.ppu.read_status(),
            MappedAddress::SprRamIoRegister => self.ppu.read_spr_ram_data(),
            MappedAddress::VramIoRegister => self.ppu.read_vram_data(),
            MappedAddress::Joypad1 => {
                let bit = self.joypad1.read() & 1;
                (self.last_bus_value & 0xfe) | bit
            }
            MappedAddress::Joypad2 => self.last_bus_value & 0xfe,
            MappedAddress::PapuSoundVerticalClockSignalRegister => self.last_bus_value & 0xe0,
            MappedAddress::ApuTestRegister | MappedAddress::PapuNoiseUnusedRegister => {
                self.last_bus_value
            }
            _ => self.last_bus_value,
        };
        self.last_bus_value = value;
        value
    }

    fn write_word(&mut self, addr: u16, value: u8) {
        if MemoryMappingInterconnect::is_expansion(addr) {
            if self.trace_expansion {
                println!(
                    "Expansion write {:04x} = {:02x} (pc={:04x} opcode={:02x})",
                    addr, value, self.last_cpu_pc, self.last_cpu_opcode
                );
            }
            self.last_bus_value = value;
            return;
        }

        self.last_bus_value = value;
        match map_addr(addr) {
            MappedAddress::Ram(addr) => self.ram[addr] = value,
            MappedAddress::PrgRam(addr) => self.prg_ram[addr] = value,
            MappedAddress::PrgRom => self.mapper.write(addr, value),
            MappedAddress::PpuControlRegister => self.ppu.write_ctrl(value),
            MappedAddress::PpuMaskRegister => self.ppu.write_mask(value),
            MappedAddress::SprRamAddressRegister => self.ppu.write_spr_ram_addr(value),
            MappedAddress::SprRamIoRegister => self.ppu.write_spr_ram_data(value),
            MappedAddress::PpuScrollRegister => self.ppu.write_scroll(value),
            MappedAddress::Joypad1 => self.joypad1.write_strobe(value),
            MappedAddress::VramAddressRegister => self.ppu.write_vram_addr(value),
            MappedAddress::VramIoRegister => self.ppu.write_vram_data(value),
            MappedAddress::PapuPulse1ControlRegister
            | MappedAddress::PapuPulse1RampControlRegister
            | MappedAddress::PapuPulse1FineTuneRegister
            | MappedAddress::PapuPulse1CoarseTuneRegister
            | MappedAddress::PapuPulse2ControlRegister
            | MappedAddress::PapuPulse2RampControlRegister
            | MappedAddress::PapuPulse2FineTuneRegister
            | MappedAddress::PapuPulse2CoarseTuneRegister
            | MappedAddress::PapuTriangleControlRegister1
            | MappedAddress::PapuTriangleControlRegister2
            | MappedAddress::PapuTriangleFrequencyRegister1
            | MappedAddress::PapuTriangleFrequencyRegister2
            | MappedAddress::PapuNoiseControlRegister1
            | MappedAddress::PapuNoiseUnusedRegister
            | MappedAddress::PapuNoiseFrequencyRegister1
            | MappedAddress::PapuNoiseFrequencyRegister2
            | MappedAddress::PapuDeltaModulationControlRegister
            | MappedAddress::PapuDeltaModulationDaRegister
            | MappedAddress::PapuDeltaModulationAddressRegister
            | MappedAddress::PapuDeltaModulationDataLengthRegister
            | MappedAddress::SpriteDmaRegister
            | MappedAddress::PpuStatusRegister
            | MappedAddress::PapuSoundVerticalClockSignalRegister
            | MappedAddress::Joypad2
            | MappedAddress::ApuTestRegister => {}
        }
    }

    fn write_double(&mut self, addr: u16, value: u16) {
        if MemoryMappingInterconnect::is_expansion(addr) {
            if self.trace_expansion {
                println!(
                    "Expansion write {:04x} = {:04x} (pc={:04x} opcode={:02x})",
                    addr, value, self.last_cpu_pc, self.last_cpu_opcode
                );
            }
            self.last_bus_value = (value >> 8) as u8;
            return;
        }

        match map_addr(addr) {
            MappedAddress::Ram(addr) => {
                self.last_bus_value = value as u8;
                self.ram[addr] = value as u8;
                self.last_bus_value = (value >> 8) as u8;
                self.ram[addr + 1] = (value >> 8) as u8;
            }
            MappedAddress::PrgRam(addr) => {
                self.last_bus_value = value as u8;
                self.prg_ram[addr] = value as u8;
                self.last_bus_value = (value >> 8) as u8;
                if addr + 1 < self.prg_ram.len() {
                    self.prg_ram[addr + 1] = (value >> 8) as u8;
                }
            }
            _ => {
                println!(
                    "WARNING: Writing to unimplemented memory address: {:x}",
                    addr
                )
            }
        }
    }

    fn trace_cpu(&mut self, pc: u16, opcode: u8) {
        self.last_cpu_pc = pc;
        self.last_cpu_opcode = opcode;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use joypad::ButtonState;
    use rom::Mirroring;

    fn dummy_rom() -> Rom {
        Rom {
            prg_rom: vec![vec![0; 0x4000]],
            chr_rom: Vec::new(),
            mapper: 0,
            mirroring: Mirroring::Horizontal,
            chr_ram_size: 0x2000,
        }
    }

    #[test]
    fn test_prg_ram_read_write() {
        let mut interconnect = MemoryMappingInterconnect::new(dummy_rom());
        interconnect.write_word(0x6000, 0x42);
        assert_eq!(interconnect.read_word(0x6000), 0x42);
    }

    #[test]
    fn test_apu_status_open_bus_bits() {
        let mut interconnect = MemoryMappingInterconnect::new(dummy_rom());
        interconnect.write_word(0x0000, 0xe0);
        assert_eq!(interconnect.read_word(0x4015), 0xe0);
    }

    #[test]
    fn test_joypad_open_bus_bits() {
        let mut interconnect = MemoryMappingInterconnect::new(dummy_rom());
        interconnect.joypad1.set_state(ButtonState {
            a: true,
            b: false,
            select: false,
            start: false,
            up: false,
            down: false,
            left: false,
            right: false,
        });

        interconnect.write_word(0x4016, 0x01);
        interconnect.write_word(0x4016, 0x00);
        interconnect.write_word(0x0000, 0xa0);
        assert_eq!(interconnect.read_word(0x4016), 0xa1);
    }
}
