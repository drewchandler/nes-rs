mod vram;

use self::vram::Vram;
use rom::Mirroring;

pub const CTRL_INCR_FLAG: u8 = 0x02;
const CTRL_SPRITE_PATTERN_FLAG: u8 = 0x08;
const CTRL_BACKGROUND_FLAG: u8 = 0x10;
const CTRL_SPRITE_SIZE_FLAG: u8 = 0x20;
const CTRL_NMI_FLAG: u8 = 0x80;
const MASK_SHOW_BG_LEFT: u8 = 0x02;
const MASK_SHOW_SPRITES_LEFT: u8 = 0x04;
const MASK_DISPLAY_BACKGROUND: u8 = 0x08;
const MASK_DISPLAY_SPRITES: u8 = 0x10;
const STATUS_SPRITE_OVERFLOW_FLAG: u8 = 0x20;
const STATUS_SPRITE0_HIT_FLAG: u8 = 0x40;
const STATUS_VBLANK_FLAG: u8 = 0x80;
const PIXELS: usize = 256 * 240;

static SYSTEM_PALETTE: [u32; 64] = [
    0x757575, 0x271b8f, 0x0000ab, 0x47009f, 0x8f0077, 0xab0013, 0xa70000, 0x7f0b00, 0x432f00,
    0x004700, 0x005100, 0x003f17, 0x1b3f5f, 0x000000, 0x000000, 0x000000, 0xbcbcbc, 0x0073ef,
    0x233bef, 0x8300f3, 0xbf00bf, 0xe7005b, 0xdb2b00, 0xcb4f0f, 0x8b7300, 0x009700, 0x00ab00,
    0x00933b, 0x00838b, 0x000000, 0x000000, 0x000000, 0xffffff, 0x3fbfff, 0x5f97ff, 0xa78bfd,
    0xf77bff, 0xff77b7, 0xff7763, 0xff9b3b, 0xf3bf3f, 0x83d313, 0x4fdf4b, 0x58f898, 0x00ebdb,
    0x000000, 0x000000, 0x000000, 0xffffff, 0xabe7ff, 0xc7d7ff, 0xd7cbff, 0xffc7ff, 0xffc7db,
    0xffbfb3, 0xffdbab, 0xffe7a3, 0xe3ffa3, 0xabf3bf, 0xb3ffcf, 0x9ffff3, 0x000000, 0x000000,
    0x000000,
];

pub struct CycleResult {
    pub end_frame: bool,
    pub nmi: bool,
}

impl CycleResult {
    fn new(end_frame: bool, nmi: bool) -> CycleResult {
        CycleResult { end_frame, nmi }
    }
}

pub struct Ppu {
    ctrl: u8,
    mask: u8,
    status: u8,
    spr_ram_addr: u8,
    fine_x: u8,
    vram_addr: u16,
    tmp_vram_addr: u16,
    write_flag: bool,
    vram: Vram,
    spr_ram: [u8; 256],
    pub screen: [u32; PIXELS],
    bg_shift_low: u16,
    bg_shift_high: u16,
    attr_shift_low: u16,
    attr_shift_high: u16,
    next_tile_id: u8,
    next_tile_attr: u8,
    next_tile_lsb: u8,
    next_tile_msb: u8,
    buffered_read: u8,
    cycle: u16,
    scanline: i16,
    nmi_queued: bool,
    open_bus: u8,
    odd_frame: bool,
    sprite_count: usize,
    sprite_x: [u8; 8],
    sprite_attr: [u8; 8],
    sprite_shift_low: [u8; 8],
    sprite_shift_high: [u8; 8],
    sprite_index: [u8; 8],
    next_sprite_count: usize,
    next_sprite_x: [u8; 8],
    next_sprite_attr: [u8; 8],
    next_sprite_shift_low: [u8; 8],
    next_sprite_shift_high: [u8; 8],
    next_sprite_index: [u8; 8],
}

impl Ppu {
    pub fn new(chr_rom: Vec<u8>, chr_ram_size: usize, mirroring: Mirroring) -> Ppu {
        Ppu {
            ctrl: 0,
            mask: 0,
            status: 0,
            spr_ram_addr: 0,
            fine_x: 0,
            vram_addr: 0,
            tmp_vram_addr: 0,
            write_flag: false,
            vram: Vram::new(chr_rom, chr_ram_size, mirroring),
            spr_ram: [0; 256],
            screen: [0; PIXELS],
            bg_shift_low: 0,
            bg_shift_high: 0,
            attr_shift_low: 0,
            attr_shift_high: 0,
            next_tile_id: 0,
            next_tile_attr: 0,
            next_tile_lsb: 0,
            next_tile_msb: 0,
            buffered_read: 0,
            cycle: 0,
            scanline: -1,
            nmi_queued: false,
            open_bus: 0,
            odd_frame: false,
            sprite_count: 0,
            sprite_x: [0; 8],
            sprite_attr: [0; 8],
            sprite_shift_low: [0; 8],
            sprite_shift_high: [0; 8],
            sprite_index: [0; 8],
            next_sprite_count: 0,
            next_sprite_x: [0; 8],
            next_sprite_attr: [0; 8],
            next_sprite_shift_low: [0; 8],
            next_sprite_shift_high: [0; 8],
            next_sprite_index: [0; 8],
        }
    }

    pub fn read_status(&mut self) -> u8 {
        let status = (self.status & 0xE0) | (self.open_bus & 0x1F);

        self.set_vblank(false);
        self.write_flag = false;
        self.open_bus = status;

        status
    }

    pub fn write_ctrl(&mut self, value: u8) {
        let prev_nmi = self.nmi_flag();
        self.ctrl = value;
        self.open_bus = value;
        self.tmp_vram_addr = self.tmp_vram_addr & 0xf3ff | ((value as u16) & 0x03) << 10;
        if !prev_nmi && self.nmi_flag() && self.status & STATUS_VBLANK_FLAG != 0 {
            self.nmi_queued = true;
        }
    }

    pub fn write_mask(&mut self, value: u8) {
        self.mask = value;
        self.open_bus = value;
    }

    pub fn write_spr_ram_addr(&mut self, value: u8) {
        self.spr_ram_addr = value;
        self.open_bus = value;
    }

    pub fn write_spr_ram_data(&mut self, value: u8) {
        self.spr_ram[self.spr_ram_addr as usize] = value;
        self.spr_ram_addr = self.spr_ram_addr.overflowing_add(1).0;
        self.open_bus = value;
    }

    pub fn read_spr_ram_data(&mut self) -> u8 {
        let value = self.spr_ram[self.spr_ram_addr as usize];
        self.open_bus = value;
        value
    }

    pub fn write_scroll(&mut self, value: u8) {
        if self.write_flag {
            self.tmp_vram_addr = (self.tmp_vram_addr & 0x0c1f)
                | ((value as u16) & 0x07) << 12
                | ((value as u16) & 0x38) << 2
                | ((value as u16) & 0xc0) << 2;
        } else {
            self.fine_x = value & 0x07;
            self.tmp_vram_addr = (self.tmp_vram_addr & 0xffe0) | (value as u16) >> 3;
        }

        self.write_flag = !self.write_flag;
        self.open_bus = value;
    }

    pub fn write_vram_addr(&mut self, value: u8) {
        if self.write_flag {
            self.tmp_vram_addr = self.tmp_vram_addr & 0xff00 | value as u16;
            self.vram_addr = self.tmp_vram_addr;
        } else {
            self.tmp_vram_addr = (self.tmp_vram_addr & 0x80FF) | ((value as u16) & 0x3F) << 8;
        }

        self.write_flag = !self.write_flag;
        self.open_bus = value;
    }

    pub fn write_vram_data(&mut self, value: u8) {
        let addr = self.vram_addr;
        self.vram.write(addr, value);
        self.increment_2007();
        self.open_bus = value;
    }

    pub fn read_vram_data(&mut self) -> u8 {
        let addr = self.vram_addr % 0x4000;
        let value = if addr >= 0x3f00 {
            self.buffered_read = self.vram.read(addr - 0x1000);
            self.vram.read(addr)
        } else {
            let buffered = self.buffered_read;
            self.buffered_read = self.vram.read(addr);
            buffered
        };

        self.increment_2007();
        self.open_bus = value;

        value
    }

    fn increment_2007(&mut self) {
        let rendering = self.rendering_enabled() && self.scanline >= 0 && self.scanline <= 239;
        if rendering {
            self.increment_y();
            return;
        }

        let by32 = self.ctrl & CTRL_INCR_FLAG != 0;
        let mut fv = (self.vram_addr >> 12) & 0x7;
        let mut v = (self.vram_addr >> 11) & 0x1;
        let mut h = (self.vram_addr >> 10) & 0x1;
        let mut vt = (self.vram_addr >> 5) & 0x1f;
        let mut ht = self.vram_addr & 0x1f;

        if by32 {
            vt += 1;
        } else {
            ht += 1;
        }

        vt += (ht >> 5) & 1;
        h += (vt >> 5) & 1;
        v += (h >> 1) & 1;
        fv += (v >> 1) & 1;

        ht &= 0x1f;
        vt &= 0x1f;
        h &= 1;
        v &= 1;
        fv &= 7;

        self.vram_addr = (fv << 12) | (v << 11) | (h << 10) | (vt << 5) | ht;
    }

    pub fn step(&mut self) -> CycleResult {
        let mut nmi = false;
        let rendering = self.rendering_enabled();

        if rendering {
            if self.scanline >= 0 && self.scanline <= 239 && self.cycle >= 1 && self.cycle <= 256 {
                self.render_pixel();
            }

            if (self.scanline >= -1 && self.scanline <= 239) || self.scanline == 261 {
                if (self.cycle >= 1 && self.cycle <= 256)
                    || (self.cycle >= 321 && self.cycle <= 336)
                {
                    self.shift_background_shifters();
                    self.fetch_background();
                }

                if self.cycle == 256 {
                    self.increment_y();
                } else if self.cycle == 257 {
                    self.copy_horizontal();
                    self.evaluate_sprites(self.scanline + 1);
                }

                if self.scanline == 261 && self.cycle >= 280 && self.cycle <= 304 {
                    self.copy_vertical();
                }
            }
        }

        if self.scanline == 261 && self.cycle == 1 {
            self.set_vblank(false);
            self.status &= !(STATUS_SPRITE0_HIT_FLAG | STATUS_SPRITE_OVERFLOW_FLAG);
        } else if self.scanline == 241 && self.cycle == 1 {
            self.set_vblank(true);
            if self.nmi_flag() {
                nmi = true;
            }
        }

        if self.nmi_queued {
            nmi = true;
            self.nmi_queued = false;
        }

        let end_frame = self.tick();
        CycleResult::new(end_frame, nmi)
    }

    fn rendering_enabled(&self) -> bool {
        self.display_background() || self.display_sprites()
    }

    fn show_bg_left(&self) -> bool {
        self.mask & MASK_SHOW_BG_LEFT != 0
    }

    fn show_sprites_left(&self) -> bool {
        self.mask & MASK_SHOW_SPRITES_LEFT != 0
    }

    fn copy_horizontal(&mut self) {
        self.vram_addr = (self.vram_addr & 0xFBE0) | (self.tmp_vram_addr & 0x041F);
    }

    fn copy_vertical(&mut self) {
        self.vram_addr = (self.vram_addr & 0x841F) | (self.tmp_vram_addr & 0x7BE0);
    }

    fn render_pixel(&mut self) {
        if self.cycle < 1 || self.cycle > 256 {
            return;
        }

        let x = (self.cycle - 1) as u8;
        let bit = 0x8000 >> self.fine_x;

        let (mut bg_pixel, mut bg_palette) = (0u8, 0u8);
        if self.display_background() && (self.show_bg_left() || x >= 8) {
            let low = if (self.bg_shift_low & bit) != 0 { 1 } else { 0 };
            let high = if (self.bg_shift_high & bit) != 0 {
                2
            } else {
                0
            };
            bg_pixel = low | high;
            let attr_low = if (self.attr_shift_low & bit) != 0 {
                1
            } else {
                0
            };
            let attr_high = if (self.attr_shift_high & bit) != 0 {
                2
            } else {
                0
            };
            bg_palette = attr_low | attr_high;
        }

        let mut color = self.map_color(self.vram.read(0x3f00));
        if bg_pixel != 0 {
            let bg_color_addr = 0x3f00 + ((bg_palette as u16) << 2) + bg_pixel as u16;
            let bg_palette_index = self.vram.read(bg_color_addr);
            color = self.map_color(bg_palette_index);
        }

        if self.display_sprites() && (self.show_sprites_left() || x >= 8) {
            if let Some((sprite_pixel, sprite_palette, behind_bg, sprite0)) = self.sprite_pixel() {
                let bg_opaque = bg_pixel != 0;
                if !behind_bg || !bg_opaque {
                    let sprite_addr = 0x3f10 + ((sprite_palette as u16) << 2) + sprite_pixel as u16;
                    let sprite_index = self.vram.read(sprite_addr);
                    color = self.map_color(sprite_index);
                }

                if sprite0
                    && bg_opaque
                    && self.display_background()
                    && ((self.show_bg_left() && self.show_sprites_left()) || x >= 8)
                {
                    self.status |= STATUS_SPRITE0_HIT_FLAG;
                }
            }
        }

        self.screen[256 * self.scanline as usize + self.cycle as usize - 1] = color;

        if self.display_sprites() {
            self.update_sprite_shifters();
        }
    }

    fn shift_background_shifters(&mut self) {
        self.bg_shift_low <<= 1;
        self.bg_shift_high <<= 1;
        self.attr_shift_low <<= 1;
        self.attr_shift_high <<= 1;
    }

    fn fetch_background(&mut self) {
        match self.cycle % 8 {
            1 => self.fetch_name_table_byte(),
            3 => self.fetch_attribute_table_byte(),
            5 => self.fetch_low_bg_tile_byte(),
            7 => self.fetch_high_bg_tile_byte(),
            0 => {
                self.load_background_shifters();
                self.increment_x();
            }
            _ => {}
        }
    }

    fn evaluate_sprites(&mut self, target_scanline: i16) {
        self.next_sprite_count = 0;

        if !(0..=239).contains(&target_scanline) {
            return;
        }

        let sprite_height = self.sprite_height() as i16;
        let mut count = 0;

        for i in 0..64 {
            let base = i * 4;
            let y = self.spr_ram[base] as i16;
            let row = target_scanline - (y + 1);
            if row >= 0 && row < sprite_height {
                count += 1;
                if self.next_sprite_count < 8 {
                    let idx = self.next_sprite_count;
                    self.next_sprite_index[idx] = i as u8;
                    self.next_sprite_attr[idx] = self.spr_ram[base + 2];
                    self.next_sprite_x[idx] = self.spr_ram[base + 3];
                    self.next_sprite_count += 1;
                }
            }
        }

        if count > 8 {
            self.status |= STATUS_SPRITE_OVERFLOW_FLAG;
        }

        for i in 0..self.next_sprite_count {
            let sprite_idx = self.next_sprite_index[i] as usize;
            let base = sprite_idx * 4;
            let y = self.spr_ram[base] as i16;
            let tile = self.spr_ram[base + 1];
            let attr = self.spr_ram[base + 2];
            let mut row = target_scanline - (y + 1);

            if attr & 0x80 != 0 {
                row = sprite_height - 1 - row;
            }

            let (tile, row_in_tile, table_base) = if sprite_height == 16 {
                let bank = tile & 0x01;
                let mut tile = tile & 0xFE;
                let mut row_in_tile = row as u8;
                if row_in_tile >= 8 {
                    tile = tile.wrapping_add(1);
                    row_in_tile -= 8;
                }
                let table_base = if bank == 0 { 0 } else { 0x1000 };
                (tile, row_in_tile, table_base)
            } else {
                (tile, row as u8, self.sprite_pattern_table())
            };

            let addr = table_base + (tile as u16) * 16 + row_in_tile as u16;
            let mut low = self.vram.read(addr);
            let mut high = self.vram.read(addr + 8);

            if attr & 0x40 != 0 {
                low = reverse_byte(low);
                high = reverse_byte(high);
            }

            self.next_sprite_shift_low[i] = low;
            self.next_sprite_shift_high[i] = high;
        }
    }

    fn sprite_pixel(&self) -> Option<(u8, u8, bool, bool)> {
        for i in 0..self.sprite_count {
            if self.sprite_x[i] != 0 {
                continue;
            }

            let low = (self.sprite_shift_low[i] & 0x80) >> 7;
            let high = (self.sprite_shift_high[i] & 0x80) >> 6;
            let pixel = low | high;
            if pixel == 0 {
                continue;
            }

            let palette = self.sprite_attr[i] & 0x03;
            let behind_bg = self.sprite_attr[i] & 0x20 != 0;
            let sprite0 = self.sprite_index[i] == 0;
            return Some((pixel, palette, behind_bg, sprite0));
        }

        None
    }

    fn update_sprite_shifters(&mut self) {
        for i in 0..self.sprite_count {
            if self.sprite_x[i] > 0 {
                self.sprite_x[i] = self.sprite_x[i].wrapping_sub(1);
            } else {
                self.sprite_shift_low[i] <<= 1;
                self.sprite_shift_high[i] <<= 1;
            }
        }
    }

    fn load_next_sprites(&mut self) {
        self.sprite_count = self.next_sprite_count;
        self.sprite_x = self.next_sprite_x;
        self.sprite_attr = self.next_sprite_attr;
        self.sprite_shift_low = self.next_sprite_shift_low;
        self.sprite_shift_high = self.next_sprite_shift_high;
        self.sprite_index = self.next_sprite_index;
    }

    fn fetch_name_table_byte(&mut self) {
        let addr = 0x2000 | self.vram_addr & 0x0fff;
        self.next_tile_id = self.vram.read(addr);
    }

    fn fetch_attribute_table_byte(&mut self) {
        let addr = 0x23C0
            | (self.vram_addr & 0x0C00)
            | ((self.vram_addr >> 4) & 0x38)
            | ((self.vram_addr >> 2) & 0x07);
        let shift = ((self.vram_addr >> 4) & 0x04) | (self.vram_addr & 0x02);
        self.next_tile_attr = (self.vram.read(addr) >> shift) & 0x03;
    }

    fn fetch_low_bg_tile_byte(&mut self) {
        let fine_y = (self.vram_addr >> 12) & 0x07;
        let tile = self.next_tile_id as u16;
        let addr = self.background_pattern_table() + tile * 16 + fine_y;
        self.next_tile_lsb = self.vram.read(addr);
    }

    fn fetch_high_bg_tile_byte(&mut self) {
        let fine_y = (self.vram_addr >> 12) & 0x07;
        let tile = self.next_tile_id as u16;
        let addr = self.background_pattern_table() + tile * 16 + fine_y + 8;
        self.next_tile_msb = self.vram.read(addr);
    }

    fn load_background_shifters(&mut self) {
        self.bg_shift_low = (self.bg_shift_low & 0xFF00) | self.next_tile_lsb as u16;
        self.bg_shift_high = (self.bg_shift_high & 0xFF00) | self.next_tile_msb as u16;

        let attr_low = if self.next_tile_attr & 0x01 != 0 {
            0xFF
        } else {
            0x00
        };
        let attr_high = if self.next_tile_attr & 0x02 != 0 {
            0xFF
        } else {
            0x00
        };

        self.attr_shift_low = (self.attr_shift_low & 0xFF00) | attr_low;
        self.attr_shift_high = (self.attr_shift_high & 0xFF00) | attr_high;
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

        if self.cycle == 340 && self.scanline == 261 && self.rendering_enabled() && self.odd_frame {
            self.cycle = 0;
            self.scanline = 0;
            self.odd_frame = !self.odd_frame;
            self.load_next_sprites();
            return true;
        }

        if self.cycle > 340 {
            self.cycle = 0;
            self.scanline += 1;

            if self.scanline > 261 {
                self.scanline = 0;
                end_frame = true;
                self.odd_frame = !self.odd_frame;
            }

            if self.scanline >= 0 && self.scanline <= 239 {
                self.load_next_sprites();
            }
        }

        end_frame
    }

    fn background_pattern_table(&self) -> u16 {
        if self.ctrl & CTRL_BACKGROUND_FLAG == 0 {
            0
        } else {
            0x1000
        }
    }

    fn sprite_pattern_table(&self) -> u16 {
        if self.ctrl & CTRL_SPRITE_PATTERN_FLAG == 0 {
            0
        } else {
            0x1000
        }
    }

    fn sprite_height(&self) -> u8 {
        if self.ctrl & CTRL_SPRITE_SIZE_FLAG == 0 {
            8
        } else {
            16
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

    fn map_color(&self, palette_index: u8) -> u32 {
        let mut index = palette_index & 0x3F;
        if self.mask & 0x01 != 0 {
            index &= 0x30;
        }
        let mut color = SYSTEM_PALETTE[index as usize];

        let emphasis = (self.mask >> 5) & 0x07;
        if emphasis != 0 {
            let mut r = ((color >> 16) & 0xFF) as u8;
            let mut g = ((color >> 8) & 0xFF) as u8;
            let mut b = (color & 0xFF) as u8;

            if emphasis & 0x01 == 0 {
                r = ((r as u16) * 3 / 4) as u8;
            }
            if emphasis & 0x02 == 0 {
                g = ((g as u16) * 3 / 4) as u8;
            }
            if emphasis & 0x04 == 0 {
                b = ((b as u16) * 3 / 4) as u8;
            }

            color = ((r as u32) << 16) | ((g as u32) << 8) | (b as u32);
        }

        color
    }

    fn set_vblank(&mut self, value: bool) {
        self.status = if value {
            self.status | STATUS_VBLANK_FLAG
        } else {
            self.status & !STATUS_VBLANK_FLAG
        };
    }
}

fn reverse_byte(mut value: u8) -> u8 {
    let mut reversed = 0u8;
    for _ in 0..8 {
        reversed = (reversed << 1) | (value & 1);
        value >>= 1;
    }
    reversed
}

#[cfg(test)]
mod tests {
    use super::*;

    fn new_test_ppu() -> Ppu {
        Ppu::new(Vec::new(), 0x2000, Mirroring::Horizontal)
    }

    #[test]
    fn test_write_ctrl() {
        let mut ppu = new_test_ppu();

        ppu.write_ctrl(0xff);

        assert_eq!(ppu.ctrl, 0xff);
        assert_eq!(ppu.tmp_vram_addr, 0x0c00);
    }

    #[test]
    fn test_write_scroll() {
        let mut ppu = new_test_ppu();

        ppu.write_scroll(0x7d);

        assert_eq!(ppu.fine_x, 0x05);
        assert_eq!(ppu.tmp_vram_addr, 0x0f);
        assert!(ppu.write_flag);

        ppu.write_scroll(0x5e);

        assert_eq!(ppu.tmp_vram_addr, 0x616f);
        assert!(!ppu.write_flag);
    }

    #[test]
    fn test_writing_to_vram() {
        let mut ppu = new_test_ppu();

        ppu.read_status();
        ppu.write_vram_addr(0x21);
        ppu.write_vram_addr(0x08);
        ppu.write_vram_data(0xff);
        ppu.write_vram_data(0xfe);

        assert_eq!(ppu.vram.read(0x2108), 0xff);
        assert_eq!(ppu.vram.read(0x2109), 0xfe);

        ppu.ctrl |= CTRL_INCR_FLAG;
        ppu.read_status();
        ppu.write_vram_addr(0x21);
        ppu.write_vram_addr(0x08);
        ppu.write_vram_data(0xaa);
        ppu.write_vram_data(0xab);

        assert_eq!(ppu.vram.read(0x2108), 0xaa);
        assert_eq!(ppu.vram.read(0x2108 + 32), 0xab);
    }

    #[test]
    fn test_ppudata_increment_during_rendering() {
        let mut ppu = new_test_ppu();
        ppu.mask = MASK_DISPLAY_BACKGROUND;
        ppu.scanline = 0;
        ppu.vram_addr = 0x0000;

        ppu.write_vram_data(0x11);

        assert_eq!(ppu.vram_addr, 0x1000);
    }

    #[test]
    fn test_writing_to_spr_ram() {
        let mut ppu = new_test_ppu();

        ppu.write_spr_ram_addr(0x08);
        ppu.write_spr_ram_data(0xff);
        ppu.write_spr_ram_data(0xfe);

        assert_eq!(ppu.spr_ram[0x08_usize], 0xff);
        assert_eq!(ppu.spr_ram[0x09_usize], 0xfe);
    }

    #[test]
    fn test_ppudata_buffered_reads() {
        let mut ppu = new_test_ppu();

        ppu.vram.write(0x2000, 0x12);
        ppu.vram.write(0x2001, 0x34);

        ppu.read_status();
        ppu.write_vram_addr(0x20);
        ppu.write_vram_addr(0x00);

        assert_eq!(ppu.read_vram_data(), 0x00);
        assert_eq!(ppu.read_vram_data(), 0x12);
    }

    #[test]
    fn test_ppudata_palette_read_bypass() {
        let mut ppu = new_test_ppu();

        ppu.vram.write(0x2f00, 0x99);
        ppu.vram.write(0x3f00, 0x3a);

        ppu.read_status();
        ppu.write_vram_addr(0x3f);
        ppu.write_vram_addr(0x00);

        assert_eq!(ppu.read_vram_data(), 0x3a);
        assert_eq!(ppu.buffered_read, 0x99);
    }

    #[test]
    fn test_oam_read_write() {
        let mut ppu = new_test_ppu();

        ppu.write_spr_ram_addr(0x10);
        ppu.write_spr_ram_data(0x7b);
        ppu.write_spr_ram_addr(0x10);

        assert_eq!(ppu.read_spr_ram_data(), 0x7b);
    }
}
