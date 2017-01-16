pub const CTRL_INCR_FLAG: u8 = 0x02;
const CTRL_NMI_FLAG: u8 = 0x80;
const STATUS_VBLANK_FLAG: u8 = 0x80;

pub struct Ppu {
    pub ctrl: u8,
    pub mask: u8,
    status: u8,
    spr_ram: [u8; 256],
    spr_addr: u8,
    scroll: u16,
    scroll_latch: Latch,
    vram: [u8; 16384],
    vram_addr: u16,
    vram_addr_latch: Latch,
    scanline: i16,
    cycle: u16,
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
            spr_ram: [0; 256],
            spr_addr: 0,
            scroll: 0,
            scroll_latch: Latch::High,
            vram: [0; 16384],
            vram_addr: 0,
            vram_addr_latch: Latch::High,
            scanline: -1,
            cycle: 0,
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
        self.vram_addr = 0;
        self.vram_addr_latch = Latch::High;
        self.scroll = 0;
        self.scroll_latch = Latch::High;

        status
    }

    pub fn set_vram_addr(&mut self, value: u8) {
        if let Latch::High = self.vram_addr_latch {
            self.vram_addr_latch = Latch::Low;
            self.vram_addr = (value as u16) << 8;
        } else {
            self.vram_addr_latch = Latch::High;
            self.vram_addr += value as u16;
        }
    }

    pub fn write_vram_data(&mut self, value: u8) {
        self.vram[self.vram_addr as usize] = value;
        let incr = self.addr_increment();
        self.vram_addr += incr;
    }

    pub fn set_spr_ram_addr(&mut self, value: u8) {
        self.spr_addr = value;
    }

    pub fn write_spr_ram_data(&mut self, value: u8) {
        self.spr_ram[self.spr_addr as usize] = value;
        self.spr_addr = self.spr_addr.overflowing_add(1).0;
    }

    pub fn set_scroll(&mut self, value: u8) {
        if let Latch::High = self.scroll_latch {
            self.scroll_latch = Latch::Low;
            self.scroll = (value as u16) << 8;
        } else {
            self.scroll_latch = Latch::High;
            self.scroll += value as u16;
        }
    }

    fn addr_increment(&self) -> u16 {
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
    fn test_writing_to_vram() {
        let mut ppu = Ppu::new();

        ppu.read_status();
        ppu.set_vram_addr(0x21);
        ppu.set_vram_addr(0x08);
        ppu.write_vram_data(0xff);
        ppu.write_vram_data(0xfe);

        assert_eq!(ppu.vram[0x2108], 0xff);
        assert_eq!(ppu.vram[0x2109], 0xfe);

        ppu.ctrl = ppu.ctrl | CTRL_INCR_FLAG;
        ppu.read_status();
        ppu.set_vram_addr(0x21);
        ppu.set_vram_addr(0x08);
        ppu.write_vram_data(0xaa);
        ppu.write_vram_data(0xab);

        assert_eq!(ppu.vram[0x2108], 0xaa);
        assert_eq!(ppu.vram[0x2108 + 32], 0xab);
    }

    #[test]
    fn test_writing_to_spr_ram() {
        let mut ppu = Ppu::new();

        ppu.set_spr_ram_addr(0x08);
        ppu.write_spr_ram_data(0xff);
        ppu.write_spr_ram_data(0xfe);

        assert_eq!(ppu.spr_ram[0x08 as usize], 0xff);
        assert_eq!(ppu.spr_ram[0x09 as usize], 0xfe);
    }


    #[test]
    fn setting_scroll_position() {
        let mut ppu = Ppu::new();

        ppu.read_status();
        ppu.set_scroll(0x21);
        ppu.set_scroll(0x08);

        assert_eq!(ppu.scroll, 0x2108);
    }
}
