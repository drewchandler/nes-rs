use rom::Mirroring;
use mapper::Cartridge;
use std::cell::Cell;

pub struct Vram {
    data: [u8; 16384],
    mirroring: Mirroring,
    cartridge: Option<Cartridge>,
    cycle: u64,
    bus_latch: Cell<u8>,
}

impl Vram {
    pub fn new() -> Vram {
        Vram::with_mirroring(Mirroring::Horizontal)
    }

    pub fn with_mirroring(mirroring: Mirroring) -> Vram {
        Vram {
            data: [0; 16384],
            mirroring: mirroring,
            cartridge: None,
            cycle: 0,
            bus_latch: Cell::new(0),
        }
    }

    pub fn attach_cartridge(&mut self, cartridge: Cartridge) {
        self.cartridge = Some(cartridge);
    }

    pub fn clock(&mut self) { self.cycle = self.cycle.wrapping_add(1); }
    pub fn address(&self, addr: u16) {
        if let Some(ref cartridge) = self.cartridge {
            cartridge.borrow_mut().ppu_address(addr & 0x3fff, self.cycle);
        }
    }

    fn map_addr(&self, addr: u16) -> usize {
        let addr = addr & 0x3fff;
        if addr < 0x2000 {
            addr as usize
        } else if addr < 0x3f00 {
            // $3000-$3EFF repeats the nametables; cartridge wiring chooses
            // which of the four logical tables share physical RAM.
            let offset = (addr - 0x2000) & 0x0fff;
            let table = offset / 0x400;
            let mirroring = self.cartridge.as_ref().and_then(|c| c.borrow().mirroring())
                .unwrap_or(self.mirroring);
            let physical = match mirroring {
                Mirroring::Horizontal => table / 2,
                Mirroring::Vertical => table % 2,
                Mirroring::FourScreen => table,
                Mirroring::SingleLower => 0,
                Mirroring::SingleUpper => 1,
            };
            (0x2000 + physical * 0x400 + (offset & 0x3ff)) as usize
        } else {
            let mut index = addr & 0x1f;
            if index >= 0x10 && index % 4 == 0 {
                index -= 0x10;
            }
            (0x3f00 + index) as usize
        }
    }

    pub fn read(&self, addr: u16) -> u8 {
        let addr = addr & 0x3fff;
        if let Some(ref cartridge) = self.cartridge {
            let mut mapper = cartridge.borrow_mut();
            if addr < 0x3f00 { mapper.ppu_address(addr, self.cycle); }
            if addr < 0x2000 {
                let value = mapper.read_chr(addr).unwrap_or(self.bus_latch.get());
                self.bus_latch.set(value);
                return value;
            }
        }
        let value = self.data[self.map_addr(addr)];
        if addr < 0x3f00 { self.bus_latch.set(value); }
        value
    }

    pub fn write(&mut self, addr: u16, value: u8) {
        let addr = addr & 0x3fff;
        if addr < 0x3f00 { self.bus_latch.set(value); }
        if let Some(ref cartridge) = self.cartridge {
            let mut mapper = cartridge.borrow_mut();
            if addr < 0x3f00 { mapper.ppu_address(addr, self.cycle); }
            if addr < 0x2000 { mapper.write_chr(addr, value); return; }
        }
        let mapped = self.map_addr(addr);
        self.data[mapped] = if mapped >= 0x3f00 {
            value & 0x3f
        } else {
            value
        };
    }
}
