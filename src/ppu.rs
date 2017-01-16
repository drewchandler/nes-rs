pub const CTRL_INCR_FLAG: u8 = 0x02;
const CTRL_NMI_FLAG: u8 = 0x80;
const STATUS_VBLANK_FLAG: u8 = 0x80;

pub struct Ppu {
    pub ctrl: u8,
    pub mask: u8,
    status: u8,
    addr: u16,
    vram: [u8; 16384],
    scanline: i16,
    cycle: u16,
    addr_latch: Latch,
}

enum Latch {
    High,
    Low,
}

impl Ppu {
    pub fn new() -> Ppu {
        Ppu {
            ctrl: 0,
            mask: 0,
            status: 0,
            addr: 0,
            vram: [0; 16384],
            scanline: -1,
            cycle: 0,
            addr_latch: Latch::High,
        }
    }

    pub fn step(&mut self) -> bool {
        let mut nmi = false;

        if self.scanline == 261 && self.cycle == 0 {
            self.set_vblank(false);
        } else if self.scanline == 241 && self.cycle == 1 {
            self.set_vblank(true);
            if self.nmi_flag() {
                nmi = true;
            }
        }

        self.tick();
        nmi
    }

    pub fn read_status(&mut self) -> u8 {
        let status = self.status;
        self.set_vblank(false);

        self.addr = 0;
        self.addr_latch = Latch::High;

        status
    }

    pub fn set_addr(&mut self, value: u8) {
        if let Latch::High = self.addr_latch {
            self.addr_latch = Latch::Low;
            self.addr = (value as u16) << 8;
        } else {
            self.addr_latch = Latch::High;
            self.addr += value as u16;
        }
    }

    pub fn write_word(&mut self, value: u8) {
        self.vram[self.addr as usize] = value;
        let incr = self.addr_increment();
        self.addr += incr;
    }

    pub fn addr_increment(&self) -> u16 {
        if self.ctrl & CTRL_INCR_FLAG == 0 {
            1
        } else {
            32
        }
    }

    fn nmi_flag(&self) -> bool {
        self.ctrl & CTRL_NMI_FLAG != 0
    }

    fn set_vblank(&mut self, value: bool) {
        self.status = if value {
            self.status | STATUS_VBLANK_FLAG
        } else {
            self.status & !STATUS_VBLANK_FLAG
        };
    }

    fn tick(&mut self) {
        self.cycle += 1;

        if self.cycle > 340 {
            self.cycle = 0;
            self.scanline += 1;

            if self.scanline > 261 {
                self.scanline = 0;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_writing_data() {
        let mut ppu = Ppu::new();

        ppu.read_status();
        ppu.set_addr(0x21);
        ppu.set_addr(0x08);
        ppu.write_word(0xff);
        ppu.write_word(0xfe);

        assert_eq!(ppu.vram[0x2108], 0xff);
        assert_eq!(ppu.vram[0x2109], 0xfe);

        ppu.ctrl = ppu.ctrl | CTRL_INCR_FLAG;
        ppu.read_status();
        ppu.set_addr(0x21);
        ppu.set_addr(0x08);
        ppu.write_word(0xaa);
        ppu.write_word(0xab);

        assert_eq!(ppu.vram[0x2108], 0xaa);
        assert_eq!(ppu.vram[0x2108 + 32], 0xab);
    }
}
