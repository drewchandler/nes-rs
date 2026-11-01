mod vram;
mod sprites;

use self::vram::Vram;
use self::sprites::SpritePipeline;
use rom::Mirroring;
use mapper::Cartridge;

pub const CTRL_INCR_FLAG: u8 = 0x04;
const CTRL_BACKGROUND_FLAG: u8 = 0x10;
const CTRL_NMI_FLAG: u8 = 0x80;
const MASK_DISPLAY_BACKGROUND: u8 = 0x08;
const MASK_DISPLAY_SPRITES: u8 = 0x10;
const STATUS_VBLANK_FLAG: u8 = 0x80;
const STATUS_SPRITE_ZERO_HIT: u8 = 0x40;
const STATUS_SPRITE_OVERFLOW: u8 = 0x20;
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
    region: ::region::Region,
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
    sprites: SpritePipeline,
    pub screen: [u32; PIXELS],
    name_table_byte: u8,
    attribute_table_byte: u8,
    low_bg_tile_byte: u8,
    high_bg_tile_byte: u8,
    tile_data: u64,
    buffered_read: u8,
    cycle: u16,
    scanline: i16,
    odd_frame: bool,
    nmi_output: bool,
    nmi_pending: bool,
    io_latch: u8,
}

impl Ppu {
    pub fn with_cartridge(mirroring: Mirroring, cartridge: Cartridge) -> Ppu {
        let mut ppu = Ppu::with_mirroring(mirroring);
        ppu.vram.attach_cartridge(cartridge);
        ppu
    }

    pub fn set_region(&mut self, region: ::region::Region) { self.region = region; }

    pub fn new() -> Ppu {
        Ppu::with_mirroring(Mirroring::Horizontal)
    }

    pub fn with_mirroring(mirroring: Mirroring) -> Ppu {
        Ppu {
            region: ::region::Region::Ntsc,
            ctrl: 0,
            mask: 0,
            status: 0,
            spr_ram_addr: 0,
            scroll: 0,
            vram_addr: 0,
            tmp_vram_addr: 0,
            write_flag: false,
            vram: Vram::with_mirroring(mirroring),
            spr_ram: [0; 256],
            sprites: SpritePipeline::new(),
            screen: [0; PIXELS],
            name_table_byte: 0,
            attribute_table_byte: 0,
            low_bg_tile_byte: 0,
            high_bg_tile_byte: 0,
            tile_data: 0,
            buffered_read: 0,
            cycle: 0,
            scanline: -1,
            odd_frame: false,
            nmi_output: false,
            nmi_pending: false,
            io_latch: 0,
        }
    }

    // CPU port accesses share a PPU I/O latch, independent of CPU open bus.
    pub fn read_register(&mut self, register: u16) -> u8 {
        self.io_latch = match register & 7 {
            2 => (self.read_status() & 0xe0) | (self.io_latch & 0x1f),
            4 => self.read_spr_ram_data(),
            7 => {
                let palette = (self.vram_addr & 0x3fff) >= 0x3f00;
                let value = self.read_vram_data();
                if palette { value | (self.io_latch & 0xc0) } else { value }
            }
            _ => self.io_latch,
        };
        self.io_latch
    }

    pub fn write_register(&mut self, register: u16, value: u8) {
        // Even a write to read-only PPUSTATUS charges the I/O latch.
        self.io_latch = value;
        match register & 7 {
            0 => self.write_ctrl(value),
            1 => self.write_mask(value),
            3 => self.write_spr_ram_addr(value),
            4 => self.write_spr_ram_data(value),
            5 => self.write_scroll(value),
            6 => self.write_vram_addr(value),
            7 => self.write_vram_data(value),
            _ => {}
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
        self.update_nmi();
        self.tmp_vram_addr = self.tmp_vram_addr & 0xf3ff | ((value as u16) & 0x03) << 10;
    }

    pub fn write_mask(&mut self, value: u8) {
        self.mask = value;
    }

    pub fn write_spr_ram_addr(&mut self, value: u8) {
        self.spr_ram_addr = value;
    }

    pub fn read_spr_ram_data(&self) -> u8 {
        self.primary_oam_byte(self.spr_ram_addr as usize)
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
            if !self.rendering_enabled() || !self.render_scanline() { self.vram.address(self.vram_addr); }
        } else {
            self.tmp_vram_addr = (self.tmp_vram_addr & 0x00ff) | ((value as u16) & 0x3F) << 8;
        }

        self.write_flag = !self.write_flag;
    }

    pub fn write_vram_data(&mut self, value: u8) {
        let addr = self.vram_addr;
        self.vram.write(addr, value);
        self.incr_vram_addr();
    }

    pub fn read_vram_data(&mut self) -> u8 {
        let addr = self.vram_addr & 0x3fff;
        let value = if addr >= 0x3f00 {
            // Palette reads bypass the buffer, but still refill it from the
            // nametable address hidden underneath palette RAM.
            self.buffered_read = self.vram.read(addr - 0x1000);
            self.vram.read(addr) & self.palette_mask()
        } else {
            let previous = self.buffered_read;
            self.buffered_read = self.vram.read(addr);
            previous
        };
        self.incr_vram_addr();
        value
    }

    fn render_scanline(&self) -> bool {
        self.scanline == -1 || self.scanline == self.region.scanlines()-1 || (self.scanline >= 0 && self.scanline < 240)
    }

    fn incr_vram_addr(&mut self) {
        if self.rendering_enabled() && self.render_scanline() {
            self.increment_x();
            self.increment_y();
        } else {
            self.vram_addr = self.vram_addr.wrapping_add(self.addr_increment()) & 0x7fff;
            self.vram.address(self.vram_addr);
        }
    }

    pub fn step(&mut self) -> CycleResult {
        self.vram.clock();
        self.finish_sprite_pixel();
        if self.scanline >= 0 && self.scanline < 240 {
            // Forced blanking still outputs a color; fetch/scroll enable and
            // visible background composition are separate decisions.
            self.process_render_scanline();
        }
        if self.rendering_enabled() && self.render_scanline() {
            self.drive_fetch_address();
            self.step_sprites();
            self.process_fetch_scanline();
            if self.cycle == 256 {
                self.increment_y();
            }
            if self.cycle == 257 {
                self.vram_addr = (self.vram_addr & !0x041f) | (self.tmp_vram_addr & 0x041f);
            }
            if (self.scanline == -1 || self.scanline == self.region.scanlines()-1) && self.cycle >= 280 && self.cycle <= 304 {
                self.vram_addr = (self.vram_addr & !0x7be0) | (self.tmp_vram_addr & 0x7be0);
            }
        }

        if (self.scanline == -1 || self.scanline == self.region.scanlines()-1) && self.cycle == 1 {
            self.set_vblank(false);
            self.status &= !(STATUS_SPRITE_ZERO_HIT | STATUS_SPRITE_OVERFLOW);
        } else if self.scanline == self.region.vblank_line() && self.cycle == 1 {
            self.set_vblank(true);
        }

        let nmi = ::std::mem::replace(&mut self.nmi_pending, false);
        let end_frame = self.tick();
        CycleResult::new(end_frame, nmi)
    }

    fn rendering_enabled(&self) -> bool {
        self.display_background() || self.display_sprites()
    }

    fn background_pixel(&self) -> u8 {
        if !self.display_background() || (self.cycle <= 8 && self.mask & 0x02 == 0) {
            return 0;
        }
        let shift = 32 + (7 - self.scroll) * 4;
        ((self.tile_data >> shift) & 0x0f) as u8
    }

    fn palette_mask(&self) -> u8 {
        if self.mask & 1 != 0 { 0x30 } else { 0x3f }
    }

    fn palette_color(&self, index: u8) -> u32 {
        let rgb = SYSTEM_PALETTE[(index & self.palette_mask()) as usize];
        let emphasis = self.mask >> 5;
        let mut r = (rgb >> 16) & 0xff;
        let mut g = (rgb >> 8) & 0xff;
        let mut b = rgb & 0xff;
        // RGB approximation for NTSC emphasis: attenuate the other channels
        // by 25%. This static palette is not an analog composite-video model.
        if emphasis & 0x06 != 0 { r = r * 3 / 4; }
        if emphasis & 0x05 != 0 { g = g * 3 / 4; }
        if emphasis & 0x03 != 0 { b = b * 3 / 4; }
        (r << 16) | (g << 8) | b
    }

    fn process_render_scanline(&mut self) {
        if self.cycle < 1 || self.cycle > 256 {
            return;
        }
        let mut pixel = self.background_pixel();
        if let Some(sprite) = self.sprite_pixel() {
            if sprite.zero && pixel & 3 != 0 && self.cycle < 256 {
                self.queue_sprite_zero_hit();
            }
            if pixel & 3 == 0 || !sprite.behind_background {
                pixel = sprite.palette;
            }
        }
        let addr = if !self.rendering_enabled() && self.vram_addr & 0x3f00 == 0x3f00 {
            // With rendering disabled, an address in palette space overrides
            // the usual universal backdrop color.
            self.vram_addr & 0x3fff
        } else if pixel & 3 == 0 {
            0x3f00
        } else {
            0x3f00 | pixel as u16
        };
        self.screen[256 * self.scanline as usize + self.cycle as usize - 1] =
            self.palette_color(self.vram.read(addr));
    }

    // The cartridge sees the address pins before the fetch latches its byte.
    // Keep that phase visible: MMC3 clocks on A12, not on the completed read.
    fn drive_fetch_address(&self) {
        let addr = match self.cycle {
            // The idle dot briefly exposes the upcoming pattern address.
            // Without this pulse, MMC3 sees a second qualified rise per line
            // when the background uses $1000 and sprites use $0000.
            0 if self.scanline >= 0 && self.scanline < 240 => Some(self.background_pattern_table() + self.name_table_byte as u16 * 16 +
                ((self.vram_addr >> 12) & 7)),
            1...255 | 320...335 => match self.cycle % 8 {
                0 => Some(0x2000 | (self.vram_addr & 0x0fff)),
                2 => Some(0x23c0 | (self.vram_addr & 0x0c00) |
                    ((self.vram_addr >> 4) & 0x38) | ((self.vram_addr >> 2) & 7)),
                4 | 6 => Some(self.background_pattern_table() + self.name_table_byte as u16 * 16 +
                    ((self.vram_addr >> 12) & 7) + if self.cycle % 8 == 6 { 8 } else { 0 }),
                _ => None,
            },
            256...319 => self.sprite_fetch_address(),
            336 | 338 => Some(0x2000 | (self.vram_addr & 0x0fff)),
            _ => None,
        };
        if let Some(addr) = addr { self.vram.address(addr); }
    }

    fn process_fetch_scanline(&mut self) {
        match self.cycle {
            1...256 | 321...336 => self.fetch_bg_data(),
            337 | 339 => { self.vram.read(0x2000 | (self.vram_addr & 0x0fff)); }
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

        // Odd rendered NTSC frames omit the last pre-render dot.
        if self.region == ::region::Region::Ntsc && self.scanline == self.region.scanlines()-1 && self.cycle == 339 && self.odd_frame && self.rendering_enabled() {
            self.cycle = 0;
            self.scanline = 0;
            self.odd_frame = false;
            return true;
        }
        self.cycle += 1;

        if self.cycle > 340 {
            self.cycle = 0;
            self.scanline += 1;

            if self.scanline >= self.region.scanlines() {
                self.scanline = 0;
                self.odd_frame = !self.odd_frame;
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

    fn update_nmi(&mut self) {
        let output = self.status & STATUS_VBLANK_FLAG != 0 && self.nmi_flag();
        if output && !self.nmi_output {
            self.nmi_pending = true;
        }
        self.nmi_output = output;
    }

    fn set_vblank(&mut self, value: bool) {
        self.status = if value {
            self.status | STATUS_VBLANK_FLAG
        } else {
            self.status & !STATUS_VBLANK_FLAG
        };
        self.update_nmi();
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

#[cfg(test)]
mod regression_tests;

state_fields!(Ppu, ctrl,mask,status,spr_ram_addr,scroll,vram_addr,tmp_vram_addr,write_flag,vram,spr_ram,sprites,screen,name_table_byte,attribute_table_byte,low_bg_tile_byte,high_bg_tile_byte,tile_data,buffered_read,cycle,scanline,odd_frame,nmi_output,nmi_pending,io_latch;|s: &Ppu| s.cycle<=340 && s.scanline>=-1 && s.scanline<s.region.scanlines() && s.scroll<8 && s.vram_addr<0x8000 && s.tmp_vram_addr<0x8000);
