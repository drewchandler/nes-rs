mod vram;

use self::vram::Vram;

pub const CTRL_INCR_FLAG: u8 = 0x02;
const CTRL_BACKGROUND_FLAG: u8 = 0x10;
const CTRL_NMI_FLAG: u8 = 0x80;
const MASK_DISPLAY_BACKGROUND: u8 = 0x08;
const MASK_DISPLAY_SPRITES: u8 = 0x10;
const STATUS_VBLANK_FLAG: u8 = 0x80;
const PIXELS: usize = 256 * 240;

static SYSTEM_PALETTE: [u32; 64] =
    [0x757575, 0x271b8f, 0x0000ab, 0x47009f, 0x8f0077, 0xab0013, 0xa70000, 0x7f0b00, 0x432f00,
     0x004700, 0x005100, 0x003f17, 0x1b3f5f, 0x000000, 0x000000, 0x000000, 0xbcbcbc, 0x0073ef,
     0x233bef, 0x8300f3, 0xbf00bf, 0xe7005b, 0xdb2b00, 0xcb4f0f, 0x8b7300, 0x009700, 0x00ab00,
     0x00933b, 0x00838b, 0x000000, 0x000000, 0x000000, 0xffffff, 0x3fbfff, 0x5f97ff, 0xa78bfd,
     0xf77bff, 0xff77b7, 0xff7763, 0xff9b3b, 0xf3bf3f, 0x83d313, 0x4fdf4b, 0x58f898, 0x00ebdb,
     0x000000, 0x000000, 0x000000, 0xffffff, 0xabe7ff, 0xc7d7ff, 0xd7cbff, 0xffc7ff, 0xffc7db,
     0xffbfb3, 0xffdbab, 0xffe7a3, 0xe3ffa3, 0xabf3bf, 0xb3ffcf, 0x9ffff3, 0x000000, 0x000000,
     0x000000];

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
    vram: Vram,
    spr_ram: [u8; 256],
    pub screen: [u32; PIXELS],
    name_table_byte: u8,
    attribute_table_byte: u8,
    low_bg_tile_byte: u8,
    high_bg_tile_byte: u8,
    tile_data: u64,
    buffered_read: u8,
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
            vram: Vram::new(),
            spr_ram: [0; 256],
            screen: [0; PIXELS],
            name_table_byte: 0,
            attribute_table_byte: 0,
            low_bg_tile_byte: 0,
            high_bg_tile_byte: 0,
            tile_data: 0,
            buffered_read: 0,
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
        let addr = self.vram_addr;
        self.vram.write(addr, value);
        self.incr_vram_addr();
    }

    pub fn read_vram_data(&mut self) -> u8 {
        let value = self.buffered_read;

        let addr = self.vram_addr;
        self.buffered_read = self.vram.read(addr);

        self.incr_vram_addr();

        value
    }

    fn incr_vram_addr(&mut self) {
        let incr = self.addr_increment();
        self.vram_addr += incr;
    }

    pub fn step(&mut self) -> CycleResult {
        let mut nmi = false;

        if self.rendering_enabled() {
            match self.scanline {
                0...239 => {
                    self.process_render_scanline();
                    self.process_fetch_scanline();

                    if self.cycle == 257 {
                        self.vram_addr = (self.vram_addr & 0xFBE0) | (self.tmp_vram_addr & 0x041F);
                    }
                }
                -1 | 261 => {
                    self.process_fetch_scanline();

                    if self.cycle >= 280 && self.cycle <= 304 {
                        self.vram_addr = (self.vram_addr & 0x841F) | (self.tmp_vram_addr & 0x7BE0);
                    }
                }
                _ => {}
            }
        }

        if self.scanline == 261 && self.cycle == 1 {
            self.set_vblank(false);
        } else if self.scanline == 241 && self.cycle == 1 {
            self.set_vblank(true);
            if self.nmi_flag() {
                nmi = true;
            }
        }

        let end_frame = self.tick();
        CycleResult::new(end_frame, nmi)
    }

    fn rendering_enabled(&self) -> bool {
        self.display_background() || self.display_sprites()
    }

    fn process_render_scanline(&mut self) {
        if self.cycle < 1 || self.cycle > 256 {
            return;
        }

        let shift = 32 + (7 - self.scroll) * 4;
        let bg_color_addr = (0x3f00 | self.tile_data >> shift) as u16;
        let bg_palette_index = self.vram.read(bg_color_addr);
        let color = SYSTEM_PALETTE[(bg_palette_index % 64) as usize];
        self.screen[256 * self.scanline as usize + self.cycle as usize - 1] = color;
    }

    fn process_fetch_scanline(&mut self) {
        match self.cycle {
            1...256 | 321...336 => self.fetch_bg_data(),
            257 => self.increment_y(),
            320 => {}
            _ => {}
        }
    }

    fn fetch_bg_data(&mut self) {
        self.tile_data <<= 4;

        match self.cycle % 8 {
            1 => self.fetch_name_table_byte(),
            3 => self.fetch_attribute_table_byte(),
            5 => self.fetch_low_bg_tile_byte(),
            7 => self.fetch_high_bg_tile_byte(),
            0 => {
                self.push_tile_data();
                self.increment_x();
            }
            _ => {}
        }
    }

    fn fetch_name_table_byte(&mut self) {
        let addr = 0x2000 | self.vram_addr & 0x0fff;
        self.name_table_byte = self.vram.read(addr);
    }

    fn fetch_attribute_table_byte(&mut self) {
        let addr = 0x23C0 | (self.vram_addr & 0x0C00) | ((self.vram_addr >> 4) & 0x38) |
                   ((self.vram_addr >> 2) & 0x07);
        let shift = ((self.vram_addr >> 4) & 0x04) | (self.vram_addr & 0x02);
        self.attribute_table_byte = ((self.vram.read(addr) >> shift) & 0x03) << 2;
    }

    fn fetch_low_bg_tile_byte(&mut self) {
        let fine_y = (self.vram_addr >> 12) & 0x07;
        let tile = self.name_table_byte as u16;
        let addr = self.background_pattern_table() + tile * 16 + fine_y;
        self.low_bg_tile_byte = self.vram.read(addr);
    }

    fn fetch_high_bg_tile_byte(&mut self) {
        let fine_y = (self.vram_addr >> 12) & 0x07;
        let tile = self.name_table_byte as u16;
        let addr = self.background_pattern_table() + tile * 16 + fine_y + 8;
        self.high_bg_tile_byte = self.vram.read(addr);
    }

    fn push_tile_data(&mut self) {
        let mut new_tile_data = 0u64;

        for _ in 0..8 {
            let low_byte = (self.low_bg_tile_byte & 0x80) >> 7;
            let high_byte = (self.high_bg_tile_byte & 0x80) >> 6;

            self.low_bg_tile_byte <<= 1;
            self.high_bg_tile_byte <<= 1;
            new_tile_data = (new_tile_data << 4) |
                            (self.attribute_table_byte | high_byte | low_byte) as u64;
        }

        self.tile_data = self.tile_data | new_tile_data;
    }

    fn increment_x(&mut self) {
        if (self.vram_addr & 0x001F) == 31 {
            self.vram_addr = self.vram_addr & !0x001F ^ 0x0400;
        } else {
            self.vram_addr += 1;
        }
    }

    fn increment_y(&mut self) {
        if (self.vram_addr & 0x7000) != 0x7000 {
            self.vram_addr += 0x1000;
        } else {
            self.vram_addr &= !0x7000;
            let mut y = (self.vram_addr & 0x03E0) >> 5;
            if y == 29 {
                y = 0;
                self.vram_addr ^= 0x0800;
            } else if y == 31 {
                y = 0;
            } else {
                y += 1;
            }
            self.vram_addr = (self.vram_addr & !0x03E0) | (y << 5);
        }
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

    fn background_pattern_table(&self) -> u16 {
        if self.ctrl & CTRL_BACKGROUND_FLAG == 0 {
            0
        } else {
            0x1000
        }
    }

    fn nmi_flag(&self) -> bool {
        self.ctrl & CTRL_NMI_FLAG != 0
    }

    fn display_background(&self) -> bool {
        self.mask & MASK_DISPLAY_BACKGROUND != 0
    }

    fn display_sprites(&self) -> bool {
        self.mask & MASK_DISPLAY_SPRITES != 0
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

        assert_eq!(ppu.vram.read(0x2108), 0xff);
        assert_eq!(ppu.vram.read(0x2109), 0xfe);

        ppu.ctrl = ppu.ctrl | CTRL_INCR_FLAG;
        ppu.read_status();
        ppu.write_vram_addr(0x21);
        ppu.write_vram_addr(0x08);
        ppu.write_vram_data(0xaa);
        ppu.write_vram_data(0xab);

        assert_eq!(ppu.vram.read(0x2108), 0xaa);
        assert_eq!(ppu.vram.read(0x2108 + 32), 0xab);
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
