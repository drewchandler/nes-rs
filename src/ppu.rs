const VBLANK_FLAG: u8 = 0x80;

pub struct Ppu {
    pub ctrl: u8,
    pub mask: u8,
    pub status: u8,
    scanline: i16,
    cycle: u16,
}

impl Ppu {
    pub fn new() -> Ppu {
        Ppu {
            ctrl: 0,
            mask: 0,
            status: 0,
            scanline: -1,
            cycle: 0,
        }
    }

    pub fn step(&mut self) -> bool {
        let mut vblank_occurred = false;

        if self.scanline == 0 && self.cycle == 0 {
            self.set_vblank(false);
        } else if self.scanline == 241 && self.cycle == 1 {
            self.set_vblank(true);
            vblank_occurred = true
        }

        self.tick();
        vblank_occurred
    }

    fn set_vblank(&mut self, value: bool) {
        self.status = if value {
            self.status | VBLANK_FLAG
        } else {
            self.status & !VBLANK_FLAG
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
