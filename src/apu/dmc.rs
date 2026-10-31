pub(super) const PERIODS: [u16; 16] = [
    428, 380, 340, 320, 286, 254, 226, 214, 190, 160, 142, 128, 106, 84, 72, 54,
];

pub(super) struct Dmc {
    pub irq_enabled: bool,
    pub irq: bool,
    pub looping: bool,
    pub rate: u8,
    pub output: u8,
    pub sample_address: u16,
    pub sample_length: u16,
    pub address: u16,
    pub remaining: u16,
    pub buffer: Option<u8>,
    pub shift: u8,
    pub bits: u8,
    pub silent: bool,
    pub timer: u16,
    pub dma_delay: u8,
}

impl Dmc {
    pub fn new() -> Dmc {
        Dmc {
            irq_enabled: false,
            irq: false,
            looping: false,
            rate: 0,
            output: 0,
            sample_address: 0xc000,
            sample_length: 1,
            address: 0xc000,
            remaining: 0,
            buffer: None,
            shift: 0,
            bits: 8,
            silent: true,
            timer: PERIODS[0] - 1,
            dma_delay: 0,
        }
    }

    pub fn write(&mut self, register: u16, value: u8) {
        match register {
            0 => {
                self.irq_enabled = value & 0x80 != 0;
                if !self.irq_enabled {
                    self.irq = false;
                }
                self.looping = value & 0x40 != 0;
                self.rate = value & 15;
            }
            1 => self.output = value & 0x7f,
            2 => self.sample_address = 0xc000 | ((value as u16) << 6),
            3 => self.sample_length = ((value as u16) << 4) | 1,
            _ => unreachable!(),
        }
    }

    fn restart(&mut self) {
        self.address = self.sample_address;
        self.remaining = self.sample_length;
    }

    pub fn enable(&mut self, enabled: bool, cycle: u64) {
        self.irq = false;
        if !enabled {
            self.remaining = 0;
            self.dma_delay = 0;
        } else if self.remaining == 0 {
            self.restart();
            // Initial load waits for the APU's next DMA request phase.
            self.dma_delay = if cycle & 1 == 0 { 2 } else { 3 };
        }
    }

    pub fn request(&self) -> Option<u16> {
        if self.remaining > 0 && self.buffer.is_none() && self.dma_delay == 0 {
            Some(self.address)
        } else {
            None
        }
    }

    pub fn complete_dma(&mut self, value: u8) {
        self.buffer = Some(value);
        self.address = if self.address == 0xffff {
            0x8000
        } else {
            self.address + 1
        };
        self.remaining -= 1;
        if self.remaining == 0 {
            if self.looping {
                self.restart();
            } else if self.irq_enabled {
                self.irq = true;
            }
        }
    }

    pub fn tick(&mut self) {
        if self.dma_delay > 0 {
            self.dma_delay -= 1;
        }
        if self.timer > 0 {
            self.timer -= 1;
            return;
        }
        self.timer = PERIODS[self.rate as usize] - 1;
        if !self.silent {
            if self.shift & 1 != 0 {
                if self.output <= 125 {
                    self.output += 2;
                }
            } else if self.output >= 2 {
                self.output -= 2;
            }
        }
        self.shift >>= 1;
        self.bits -= 1;
        if self.bits == 0 {
            self.bits = 8;
            match self.buffer.take() {
                Some(value) => {
                    self.shift = value;
                    self.silent = false;
                }
                None => self.silent = true,
            }
        }
    }
}
