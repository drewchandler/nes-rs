use joypad::Joypad;
use mapper::nrom::Nrom;
use mapper::unrom::Unrom;
use mapper::Mapper;
use ppu::Ppu;
use rom::Rom;

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
    pub ppu: Ppu,
    pub joypad1: Joypad,
    last_cpu_pc: u16,
    last_cpu_opcode: u8,
    last_zp_f0: Option<PtrWrite>,
    last_zp_f1: Option<PtrWrite>,
}

struct PtrWrite {
    value: u8,
    pc: u16,
    opcode: u8,
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

        let mapper: Box<dyn Mapper> = match mapper {
            0 => Box::new(Nrom::new(prg_rom)),
            2 => Box::new(Unrom::new(prg_rom)),
            _ => panic!("Unimplemented mapper"),
        };

        MemoryMappingInterconnect {
            mapper,
            ram: [0; 2048],
            ppu: Ppu::new(chr_rom, chr_ram_size, mirroring),
            joypad1: Joypad::new(),
            last_cpu_pc: 0,
            last_cpu_opcode: 0,
            last_zp_f0: None,
            last_zp_f1: None,
        }
    }

    fn check_expansion_access(&self, addr: u16, is_write: bool) {
        if (0x4020..=0x5fff).contains(&addr) {
            let access = if is_write { "write" } else { "read" };
            let f0 = self.ram[0x00f0];
            let f1 = self.ram[0x00f1];
            let f0_write = Self::format_ptr_write(&self.last_zp_f0);
            let f1_write = Self::format_ptr_write(&self.last_zp_f1);
            panic!(
                "Expansion {} at {:04x} (pc={:04x} opcode={:02x}) zp_f0={:02x} {} zp_f1={:02x} {}",
                access, addr, self.last_cpu_pc, self.last_cpu_opcode, f0, f0_write, f1, f1_write
            );
        }
    }

    fn record_zp_pointer_write(&mut self, addr: u16, value: u8) {
        let mirror = (addr % 0x800) as u16;
        let entry = PtrWrite {
            value,
            pc: self.last_cpu_pc,
            opcode: self.last_cpu_opcode,
        };
        match mirror {
            0x00f0 => self.last_zp_f0 = Some(entry),
            0x00f1 => self.last_zp_f1 = Some(entry),
            _ => {}
        }
    }

    fn format_ptr_write(entry: &Option<PtrWrite>) -> String {
        match *entry {
            Some(ref entry) => format!(
                "(last write pc {:04x} opcode {:02x} value {:02x})",
                entry.pc, entry.opcode, entry.value
            ),
            None => "(no write recorded)".to_string(),
        }
    }
}

impl Interconnect for MemoryMappingInterconnect {
    fn read_double(&mut self, addr: u16) -> u16 {
        ((self.read_word(addr + 1) as u16) << 8) + self.read_word(addr) as u16
    }

    fn read_word(&mut self, addr: u16) -> u8 {
        self.check_expansion_access(addr, false);
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
        self.check_expansion_access(addr, true);
        match map_addr(addr) {
            MappedAddress::Ram(addr) => {
                self.record_zp_pointer_write(addr as u16, value);
                self.ram[addr] = value;
            }
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
                println!(
                    "WARNING: Writing to unimplemented memory address: {:x}",
                    addr
                )
            }
        }
    }

    fn write_double(&mut self, addr: u16, value: u16) {
        self.check_expansion_access(addr, true);
        match map_addr(addr) {
            MappedAddress::Ram(addr) => {
                self.record_zp_pointer_write(addr as u16, value as u8);
                self.record_zp_pointer_write(addr as u16 + 1, (value >> 8) as u8);
                self.ram[addr] = value as u8;
                self.ram[addr + 1] = (value >> 8) as u8;
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
