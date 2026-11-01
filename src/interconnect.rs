use apu::Apu;
use joypad::Joypad;
use mapper::{self, Cartridge};
use ppu::Ppu;
use rom::Rom;
use std::cell::RefCell;
use std::rc::Rc;

pub trait Interconnect {
    fn read_double(&mut self, addr: u16) -> u16;
    fn read_word(&mut self, addr: u16) -> u8;
    fn write_word(&mut self, addr: u16, value: u8);
    fn write_double(&mut self, addr: u16, value: u16);

    // CPU accesses consume a bus cycle; debugger/setup accesses above do not.
    // Defaults keep small CPU-only test buses independent of peripherals.
    fn cpu_read(&mut self, addr: u16) -> u8 {
        self.read_word(addr)
    }
    fn cpu_write(&mut self, addr: u16, value: u8) {
        self.write_word(addr, value);
    }
    fn oam_dma_read(&mut self, addr: u16) -> u8 {
        self.cpu_read(addr)
    }
    fn take_stall_cycles(&mut self) -> u16 {
        0
    }
    fn poll_irq(&self) -> bool {
        false
    }
    fn take_nmi(&mut self) -> bool {
        false
    }
    fn reset_cpu(&mut self) {}
}

pub struct MemoryMappingInterconnect {
    region: ::region::Region,
    ppu_remainder: u8,
    mapper: Cartridge,
    ram: [u8; 2048],
    open_bus: u8,
    pub ppu: Ppu,
    pub joypad1: Joypad,
    pub joypad2: Joypad,
    pub apu: Apu,
    pub cpu_cycles: u64,
    pending_nmi: bool,
    frame_ready: bool,
    stalled_cycles: u16,
    irq_sample: bool,
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
    ApuRegister,
    ApuStatus,
    SpriteDmaRegister,
    Joypad1,
    Joypad2,
    PrgRom,
    CartridgeLow,
    OpenBus,
}

fn map_addr(addr: u16) -> MappedAddress {
    match addr {
        0x0000...0x1fff => MappedAddress::Ram(addr as usize % 2048),
        0x8000...0xffff => MappedAddress::PrgRom,
        0x4020..=0x7fff => MappedAddress::CartridgeLow,
        0x2000...0x3fff => match (addr - 0x2000) % 8 {
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
        0x4000..=0x4013 => MappedAddress::ApuRegister,
        0x4014 => MappedAddress::SpriteDmaRegister,
        0x4015 => MappedAddress::ApuStatus,
        0x4016 => MappedAddress::Joypad1,
        0x4017 => MappedAddress::Joypad2,
        _ => MappedAddress::OpenBus,
    }
}

impl MemoryMappingInterconnect {
    pub fn new(rom: Rom) -> MemoryMappingInterconnect {
        let mirroring = rom.mirroring;
        let region = rom.region;
        let mapper = mapper::create(rom);
        let mapper = Rc::new(RefCell::new(mapper));
        let mut ppu = Ppu::with_cartridge(mirroring, mapper.clone());
        ppu.set_region(region);

        MemoryMappingInterconnect {
            region,
            ppu_remainder: 0,
            mapper: mapper,
            ram: [0; 2048],
            open_bus: 0,
            ppu: ppu,
            joypad1: Joypad::new(),
            joypad2: Joypad::new(),
            apu: Apu::with_region(region),
            cpu_cycles: 0,
            pending_nmi: false,
            frame_ready: false,
            stalled_cycles: 0,
            irq_sample: false,
        }
    }

    fn clock_cpu(&mut self) {
        // IRQ is polled before the instruction's final cycle. Preserve the
        // line at the end of the preceding cycle, even if $4015 clears it.
        self.irq_sample = self.apu.irq() || self.mapper.borrow().irq();
        self.cpu_cycles = self.cpu_cycles.wrapping_add(1);
        let expansion = {
            let mut mapper = self.mapper.borrow_mut();
            mapper.clock_cpu();
            mapper.audio()
        };
        self.apu.step_with_audio(expansion);
        let dots = if self.region == ::region::Region::Pal {
            self.ppu_remainder += 16;
            let dots = self.ppu_remainder / 5;
            self.ppu_remainder %= 5;
            dots
        } else {
            3
        };
        for _ in 0..dots {
            let result = self.ppu.step();
            self.pending_nmi |= result.nmi;
            self.frame_ready |= result.end_frame;
        }
    }

    pub fn take_frame_ready(&mut self) -> bool {
        ::std::mem::replace(&mut self.frame_ready, false)
    }

    fn service_dmc(&mut self, halted_address: u16) {
        if let Some(address) = self.apu.dmc_request() {
            let start = self.cpu_cycles;
            // Halt, dummy, optional alignment, then the sample get on an even
            // CPU cycle. Only reads halt; intervening CPU writes run normally.
            self.clock_cpu();
            self.read_word(halted_address);
            self.clock_cpu();
            self.read_word(halted_address);
            if (self.cpu_cycles + 1) & 1 != 0 {
                self.clock_cpu();
                self.read_word(halted_address);
            }
            self.clock_cpu();
            debug_assert_eq!(
                self.cpu_cycles & 1,
                0,
                "DMC gets must use an even CPU cycle"
            );
            let value = self.read_word(address);
            self.apu.complete_dmc_dma(value);
            self.stalled_cycles += (self.cpu_cycles - start) as u16;
        }
    }
}

impl Interconnect for MemoryMappingInterconnect {
    fn read_double(&mut self, addr: u16) -> u16 {
        let low = self.read_word(addr) as u16;
        low | ((self.read_word(addr.wrapping_add(1)) as u16) << 8)
    }

    fn read_word(&mut self, addr: u16) -> u8 {
        let value = match map_addr(addr) {
            MappedAddress::Ram(addr) => self.ram[addr],
            MappedAddress::PrgRom => self.mapper.borrow().read(addr),
            MappedAddress::CartridgeLow => {
                self.mapper.borrow().read_low(addr).unwrap_or(self.open_bus)
            }
            MappedAddress::PpuControlRegister
            | MappedAddress::PpuMaskRegister
            | MappedAddress::PpuStatusRegister
            | MappedAddress::SprRamAddressRegister
            | MappedAddress::SprRamIoRegister
            | MappedAddress::PpuScrollRegister
            | MappedAddress::VramAddressRegister
            | MappedAddress::VramIoRegister => self.ppu.read_register(addr),
            MappedAddress::Joypad1 => (self.open_bus & 0xe0) | self.joypad1.read(),
            MappedAddress::Joypad2 => (self.open_bus & 0xe0) | self.joypad2.read(),
            MappedAddress::ApuStatus => self.apu.read_status(self.open_bus),
            // Write-only APU/expansion reads leave the CPU data bus driven
            // by its previous value. In particular, dummy reads must not panic.
            _ => self.open_bus,
        };
        self.open_bus = value;
        value
    }

    fn write_word(&mut self, addr: u16, value: u8) {
        self.open_bus = value;
        match map_addr(addr) {
            MappedAddress::Ram(addr) => self.ram[addr] = value,
            MappedAddress::PrgRom => self.mapper.borrow_mut().write(addr, value),
            MappedAddress::CartridgeLow => self.mapper.borrow_mut().write_low(addr, value),
            MappedAddress::PpuControlRegister
            | MappedAddress::PpuMaskRegister
            | MappedAddress::PpuStatusRegister
            | MappedAddress::SprRamAddressRegister
            | MappedAddress::SprRamIoRegister
            | MappedAddress::PpuScrollRegister
            | MappedAddress::VramAddressRegister
            | MappedAddress::VramIoRegister => self.ppu.write_register(addr, value),
            // $4017 reads controller 2, but writes the APU frame counter.
            MappedAddress::ApuRegister | MappedAddress::ApuStatus | MappedAddress::Joypad2 => {
                self.apu.write_register(addr, value);
            }
            MappedAddress::Joypad1 => {
                self.joypad1.strobe(value);
                self.joypad2.strobe(value);
            }
            // The CPU schedules the actual DMA transfer at its next read.
            MappedAddress::SpriteDmaRegister | MappedAddress::OpenBus => {}
        }
    }

    fn write_double(&mut self, addr: u16, value: u16) {
        match map_addr(addr) {
            MappedAddress::Ram(addr) => {
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

    fn cpu_read(&mut self, addr: u16) -> u8 {
        self.service_dmc(addr);
        self.clock_cpu();
        self.read_word(addr)
    }

    fn cpu_write(&mut self, addr: u16, value: u8) {
        self.clock_cpu();
        if addr >= 0x8000 {
            self.open_bus = value;
            self.mapper
                .borrow_mut()
                .write_cpu(addr, value, self.cpu_cycles);
        } else {
            self.write_word(addr, value);
        }
    }

    fn oam_dma_read(&mut self, addr: u16) -> u8 {
        debug_assert_eq!(
            (self.cpu_cycles + 1) & 1,
            0,
            "OAM gets must use an even CPU cycle"
        );
        if let Some(address) = self.apu.dmc_request() {
            // OAM has already halted the CPU. A DMC get replaces one OAM get;
            // the matching put is idle and OAM retries on the next get phase.
            self.clock_cpu();
            let value = self.read_word(address);
            self.apu.complete_dmc_dma(value);
            self.clock_cpu();
            self.stalled_cycles += 2;
        }
        self.clock_cpu();
        self.read_word(addr)
    }

    fn take_stall_cycles(&mut self) -> u16 {
        ::std::mem::replace(&mut self.stalled_cycles, 0)
    }

    fn poll_irq(&self) -> bool {
        self.irq_sample
    }
    fn take_nmi(&mut self) -> bool {
        ::std::mem::replace(&mut self.pending_nmi, false)
    }
    fn reset_cpu(&mut self) {
        self.apu.reset();
        self.pending_nmi = false;
        self.irq_sample = false;
        self.stalled_cycles = 0;
    }
}

impl ::state::Snapshot for MemoryMappingInterconnect {
    fn save(&self, w: &mut ::state::Writer) {
        use state::Snapshot;
        self.ram.save(w);
        self.open_bus.save(w);
        self.ppu.save(w);
        self.joypad1.save(w);
        self.joypad2.save(w);
        self.apu.save(w);
        self.cpu_cycles.save(w);
        self.pending_nmi.save(w);
        self.frame_ready.save(w);
        self.stalled_cycles.save(w);
        self.irq_sample.save(w);
        self.ppu_remainder.save(w);
        self.mapper.borrow().save(w);
    }
    fn load(&mut self, r: &mut ::state::Reader) -> ::state::Result<()> {
        use state::Snapshot;
        self.ram.load(r)?;
        self.open_bus.load(r)?;
        self.ppu.load(r)?;
        self.joypad1.load(r)?;
        self.joypad2.load(r)?;
        self.apu.load(r)?;
        self.cpu_cycles.load(r)?;
        self.pending_nmi.load(r)?;
        self.frame_ready.load(r)?;
        self.stalled_cycles.load(r)?;
        self.irq_sample.load(r)?;
        self.ppu_remainder.load(r)?;
        if self.ppu_remainder >= 5 {
            return Err("Invalid PPU clock phase".into());
        }
        self.mapper.borrow_mut().load(r)
    }
}
impl MemoryMappingInterconnect {
    pub fn battery(&self) -> Vec<u8> {
        self.mapper.borrow().battery()
    }
    pub fn load_battery(&mut self, bytes: &[u8]) -> ::state::Result<()> {
        self.mapper.borrow_mut().load_battery(bytes)
    }
}
