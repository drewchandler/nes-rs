pub const CTRL_INCR_FLAG: u8 = 0x02;
const CTRL_NMI_FLAG: u8 = 0x80;
const STATUS_VBLANK_FLAG: u8 = 0x80;
const PIXELS: usize = 256 * 240;

pub struct CycleResult {
    pub end_frame: bool,
    pub nmi: bool,
}

impl CycleResult {
    fn new(end_frame: bool, nmi: bool) -> CycleResult {
        CycleResult {
            end_frame: end_frame,
            nmi: nmi,
        }
    }
}

pub struct Ppu {
    ctrl: u8,
    mask: u8,
    status: u8,
    spr_ram_addr: u8,
    scroll: u8,
    vram_addr: u16,
    tmp_vram_addr: u16,
    write_flag: bool,
    vram: [u8; 16384],
    spr_ram: [u8; 256],
    pub screen: [u32; PIXELS],
    cycle: u16,
    scanline: i16,
}

impl Ppu {
    pub fn new() -> Ppu {
        Ppu {
            ctrl: 0,
            mask: 0,
            status: 0,
            spr_ram_addr: 0,
            scroll: 0,
            vram_addr: 0,
            tmp_vram_addr: 0,
            write_flag: false,
            vram: [0; 16384],
            spr_ram: [0; 256],
            screen: [0; PIXELS],
            cycle: 0,
            scanline: -1,
        }
    }

    pub fn read_status(&mut self) -> u8 {
        let status = self.status;

        self.set_vblank(false);
        self.write_flag = false;

        status
    }

    pub fn write_ctrl(&mut self, value: u8) {
        self.ctrl = value;
        self.tmp_vram_addr = self.tmp_vram_addr & 0xf3ff | ((value as u16) & 0x03) << 10;
    }

    pub fn write_mask(&mut self, value: u8) {
        self.mask = value;
    }

    pub fn write_spr_ram_addr(&mut self, value: u8) {
        self.spr_ram_addr = value;
    }

    pub fn write_spr_ram_data(&mut self, value: u8) {
        self.spr_ram[self.spr_ram_addr as usize] = value;
        self.spr_ram_addr = self.spr_ram_addr.overflowing_add(1).0;
    }

    pub fn write_scroll(&mut self, value: u8) {
        if self.write_flag {
            self.tmp_vram_addr = (self.tmp_vram_addr & 0x0c1f) | ((value as u16) & 0x07) << 12 |
                                 ((value as u16) & 0x38) << 2 |
                                 ((value as u16) & 0xc0) << 2;
        } else {
            self.scroll = value & 0x07;
            self.tmp_vram_addr = (self.tmp_vram_addr & 0xffe0) | (value as u16) >> 3;
        }

        self.write_flag = !self.write_flag;
    }

    pub fn write_vram_addr(&mut self, value: u8) {
        if self.write_flag {
            self.tmp_vram_addr = self.tmp_vram_addr & 0xff00 | value as u16;
            self.vram_addr = self.tmp_vram_addr;
        } else {
            self.tmp_vram_addr = (self.tmp_vram_addr & 0x80FF) | ((value as u16) & 0x3F) << 8;
        }

        self.write_flag = !self.write_flag;
    }

    pub fn write_vram_data(&mut self, value: u8) {
        self.vram[self.vram_addr as usize] = value;
        let incr = self.addr_increment();
        self.vram_addr += incr;
    }

    pub fn step(&mut self) -> CycleResult {
        let mut nmi = false;

        match self.scanline {
            241 => {
                if self.cycle == 1 {
                    self.set_vblank(true);
                    if self.nmi_flag() {
                        nmi = true;
                    }
                }
            }
            261 => {
                if self.cycle == 1 {
                    self.set_vblank(false);
                }
            }
            _ => {}
        }

        let end_frame = self.tick();
        CycleResult::new(end_frame, nmi)
    }

    fn tick(&mut self) -> bool {
        let mut end_frame = false;

        self.cycle += 1;

        if self.cycle > 340 {
            self.cycle = 0;
            self.scanline += 1;

            if self.scanline > 261 {
                self.scanline = 0;
                end_frame = true;
            }
        }

        end_frame
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_write_ctrl() {
        let mut ppu = Ppu::new();

        ppu.write_ctrl(0xff);

        assert_eq!(ppu.ctrl, 0xff);
        assert_eq!(ppu.tmp_vram_addr, 0x0c00);
    }

    #[test]
    fn test_write_scroll() {
        let mut ppu = Ppu::new();

        ppu.write_scroll(0x7d);

        assert_eq!(ppu.scroll, 0x05);
        assert_eq!(ppu.tmp_vram_addr, 0x0f);
        assert!(ppu.write_flag);

        ppu.write_scroll(0x5e);

        assert_eq!(ppu.tmp_vram_addr, 0x616f);
        assert!(!ppu.write_flag);
    }

    #[test]
    fn test_writing_to_vram() {
        let mut ppu = Ppu::new();

        ppu.read_status();
        ppu.write_vram_addr(0x21);
        ppu.write_vram_addr(0x08);
        ppu.write_vram_data(0xff);
        ppu.write_vram_data(0xfe);

        assert_eq!(ppu.vram[0x2108], 0xff);
        assert_eq!(ppu.vram[0x2109], 0xfe);

        ppu.ctrl = ppu.ctrl | CTRL_INCR_FLAG;
        ppu.read_status();
        ppu.write_vram_addr(0x21);
        ppu.write_vram_addr(0x08);
        ppu.write_vram_data(0xaa);
        ppu.write_vram_data(0xab);

        assert_eq!(ppu.vram[0x2108], 0xaa);
        assert_eq!(ppu.vram[0x2108 + 32], 0xab);
    }

    #[test]
    fn test_writing_to_spr_ram() {
        let mut ppu = Ppu::new();

        ppu.write_spr_ram_addr(0x08);
        ppu.write_spr_ram_data(0xff);
        ppu.write_spr_ram_data(0xfe);

        assert_eq!(ppu.spr_ram[0x08 as usize], 0xff);
        assert_eq!(ppu.spr_ram[0x09 as usize], 0xfe);
    }
}
