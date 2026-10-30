// Sprite evaluation and pattern fetches run ahead of the pixels they supply:
// scanline N selects the first eight OAM entries for scanline N + 1.
use super::{Ppu, STATUS_SPRITE_OVERFLOW, STATUS_SPRITE_ZERO_HIT};

#[derive(Clone, Copy)]
struct Sprite {
    x: u8,
    attributes: u8,
    low: u8,
    high: u8,
    zero: bool,
}

const EMPTY_SPRITE: Sprite = Sprite {
    x: 0,
    attributes: 0,
    low: 0,
    high: 0,
    zero: false,
};

pub(super) struct SpritePipeline {
    secondary: [u8; 32],
    indices: [u8; 8],
    count: usize,
    primary_index: usize,
    byte_index: usize,
    latch: u8,
    done: bool,
    active: [Sprite; 8],
    active_count: usize,
    pattern_addr: u16,
    hit_pending: bool,
}

impl SpritePipeline {
    pub(super) fn new() -> SpritePipeline {
        SpritePipeline {
            secondary: [0xff; 32],
            indices: [0xff; 8],
            count: 0,
            primary_index: 0,
            byte_index: 0,
            latch: 0xff,
            done: false,
            active: [EMPTY_SPRITE; 8],
            active_count: 0,
            pattern_addr: 0,
            hit_pending: false,
        }
    }
}

pub(super) struct SpritePixel {
    pub(super) palette: u8,
    pub(super) behind_background: bool,
    pub(super) zero: bool,
}

impl Ppu {
    fn sprite_height(&self) -> i16 {
        if self.ctrl & 0x20 != 0 {
            16
        } else {
            8
        }
    }

    fn sprite_in_range(&self, y: u8) -> bool {
        // OAM Y is the top scanline minus one, with no wrap onto line zero.
        // Pre-render never selects a sprite for the first visible scanline.
        let row = self.scanline - y as i16;
        self.scanline >= 0 && self.scanline < 240 && row >= 0 && row < self.sprite_height()
    }

    pub(super) fn primary_oam_byte(&self, addr: usize) -> u8 {
        let value = self.spr_ram[addr & 0xff];
        // Attribute bits 2-4 are not physically stored by OAM.
        if addr & 3 == 2 {
            value & 0xe3
        } else {
            value
        }
    }

    pub(super) fn finish_sprite_pixel(&mut self) {
        // A collision becomes observable on the dot after its pixel.
        if self.sprites.hit_pending {
            self.status |= STATUS_SPRITE_ZERO_HIT;
            self.sprites.hit_pending = false;
        }
    }

    pub(super) fn queue_sprite_zero_hit(&mut self) {
        self.sprites.hit_pending = true;
    }

    pub(super) fn sprite_pixel(&self) -> Option<SpritePixel> {
        if !self.display_sprites() || (self.cycle <= 8 && self.mask & 0x04 == 0) {
            return None;
        }
        let x = self.cycle - 1;
        for sprite in &self.sprites.active[..self.sprites.active_count] {
            if x < sprite.x as u16 || x >= sprite.x as u16 + 8 {
                continue;
            }
            let column = (x - sprite.x as u16) as u8;
            let bit = if sprite.attributes & 0x40 != 0 {
                column
            } else {
                7 - column
            };
            let color = ((sprite.low >> bit) & 1) | (((sprite.high >> bit) & 1) << 1);
            if color != 0 {
                // Select the first opaque OAM entry BEFORE applying background
                // priority: a hidden early sprite still masks later sprites.
                return Some(SpritePixel {
                    palette: 0x10 | ((sprite.attributes & 3) << 2) | color,
                    behind_background: sprite.attributes & 0x20 != 0,
                    zero: sprite.zero,
                });
            }
        }
        None
    }

    pub(super) fn step_sprites(&mut self) {
        match self.cycle {
            1...64 => {
                if self.cycle % 2 == 0 {
                    self.sprites.secondary[(self.cycle / 2 - 1) as usize] = 0xff;
                }
            }
            65...256 => self.evaluate_sprite_dot(),
            257...320 => self.fetch_sprite_dot(),
            _ => {}
        }
    }

    fn evaluate_sprite_dot(&mut self) {
        if self.cycle == 65 {
            self.sprites.count = 0;
            self.sprites.primary_index = 0;
            self.sprites.byte_index = 0;
            self.sprites.indices = [0xff; 8];
            self.sprites.done = false;
        }
        if self.sprites.done {
            return;
        }
        if self.cycle % 2 != 0 {
            let addr = self.sprites.primary_index * 4 + self.sprites.byte_index;
            self.sprites.latch = self.primary_oam_byte(addr);
            return;
        }
        let value = self.sprites.latch;
        if self.sprites.count < 8 {
            let dest = self.sprites.count * 4 + self.sprites.byte_index;
            self.sprites.secondary[dest] = value;
            if self.sprites.byte_index == 0 && !self.sprite_in_range(value) {
                self.sprites.primary_index += 1;
            } else {
                if self.sprites.byte_index == 0 {
                    self.sprites.indices[self.sprites.count] = self.sprites.primary_index as u8;
                }
                self.sprites.byte_index += 1;
                if self.sprites.byte_index == 4 {
                    self.sprites.byte_index = 0;
                    self.sprites.primary_index += 1;
                    self.sprites.count += 1;
                }
            }
        } else if self.sprite_in_range(value) {
            self.status |= STATUS_SPRITE_OVERFLOW;
            // Once overflow is latched, further evaluation cannot change this
            // scanline's selected sprites or its status result.
            self.sprites.done = true;
        } else {
            // Hardware's overflow bug scans diagonally through subsequent
            // entries: Y, tile, attributes, X, Y... instead of Y alone.
            self.sprites.primary_index += 1;
            self.sprites.byte_index = (self.sprites.byte_index + 1) & 3;
        }
        if self.sprites.primary_index == 64 {
            self.sprites.done = true;
        }
    }

    fn sprite_pattern_addr(&self, tile: u8, attributes: u8, y: u8) -> u16 {
        let height = self.sprite_height() as u8;
        let mut row = (self.scanline as u8).wrapping_sub(y) & (height - 1);
        if attributes & 0x80 != 0 {
            row = height - 1 - row;
        }
        if height == 16 {
            // Tall sprites choose the bank with tile bit 0, and use an even /
            // odd tile pair. PPUCTRL's 8x8 sprite bank bit is ignored.
            ((tile as u16 & 1) << 12)
                + ((tile as u16 & 0xfe) + (row / 8) as u16) * 16
                + (row & 7) as u16
        } else {
            (if self.ctrl & 8 != 0 { 0x1000 } else { 0 }) + tile as u16 * 16 + row as u16
        }
    }

    fn fetch_sprite_dot(&mut self) {
        self.spr_ram_addr = 0;
        let slot = ((self.cycle - 257) / 8) as usize;
        let phase = (self.cycle - 257) % 8;
        let base = slot * 4;
        if self.cycle == 257 {
            self.sprites.active_count = self.sprites.count;
            self.sprites.active = [EMPTY_SPRITE; 8];
        }
        match phase {
            0 => {
                self.sprites.active[slot].x = self.sprites.secondary[base + 3];
                self.sprites.active[slot].attributes = self.sprites.secondary[base + 2];
                self.sprites.active[slot].zero = self.sprites.indices[slot] == 0;
                self.vram.read(0x2000 | (self.vram_addr & 0x0fff));
            }
            2 => {
                self.vram.read(0x2000 | (self.vram_addr & 0x0fff));
            }
            4 => {
                let addr = self.sprite_pattern_addr(
                    self.sprites.secondary[base + 1],
                    self.sprites.secondary[base + 2],
                    self.sprites.secondary[base],
                );
                self.sprites.pattern_addr = addr;
                self.sprites.active[slot].low = self.vram.read(addr);
            }
            6 => {
                self.sprites.active[slot].high = self.vram.read(self.sprites.pattern_addr + 8);
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod rom_tests;
