use rom::Mirroring;

pub struct Vram {
    data: [u8; 16384],
    mirroring: Mirroring,
}

impl Vram {
    pub fn new() -> Vram {
        Vram::with_mirroring(Mirroring::Horizontal)
    }

    pub fn with_mirroring(mirroring: Mirroring) -> Vram {
        Vram {
            data: [0; 16384],
            mirroring: mirroring,
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
            let physical = match self.mirroring {
                Mirroring::Horizontal => table / 2,
                Mirroring::Vertical => table % 2,
                Mirroring::FourScreen => table,
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
        self.data[self.map_addr(addr)]
    }

    pub fn write(&mut self, addr: u16, value: u8) {
        let mapped = self.map_addr(addr);
        self.data[mapped] = if mapped >= 0x3f00 {
            value & 0x3f
        } else {
            value
        };
    }
}
