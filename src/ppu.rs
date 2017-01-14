const CTRL_NMI_FLAG: u8 = 0x80;
const STATUS_VBLANK_FLAG: u8 = 0x80;

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
        } else if self.scanline == 241 && self.cycle == 1 && self.nmi_flag() {
            self.set_vblank(true);
            vblank_occurred = true
        }

        self.tick();
        vblank_occurred
    }

    pub fn read_status(&mut self) -> u8 {
        let status = self.status;
        self.set_vblank(false);
        status
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
