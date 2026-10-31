// NTSC 2A03 channel units. Timers keep running when a channel is disabled;
// $4015 gates length loading, not the dividers or the held triangle/DMC DACs.
pub(super) const LENGTHS: [u8; 32] = [
    10, 254, 20, 2, 40, 4, 80, 6, 160, 8, 60, 10, 14, 12, 26, 14, 12, 16, 24, 18, 48, 20, 96, 22,
    192, 24, 72, 26, 16, 28, 32, 30,
];

#[derive(Default)]
pub(super) struct Length {
    pub enabled: bool,
    pub halt: bool,
    pub value: u8,
    pub before_clock: u8,
    pending_halt: Option<bool>,
    pending_reload: Option<u8>,
}

impl Length {
    pub fn enable(&mut self, enabled: bool) {
        self.enabled = enabled;
        if !enabled {
            self.value = 0;
            self.pending_reload = None;
        }
    }

    pub fn set_halt(&mut self, halt: bool) {
        self.pending_halt = Some(halt);
    }

    pub fn load(&mut self, value: u8) {
        if self.enabled {
            self.pending_reload = Some(value);
        }
    }

    pub fn commit(&mut self, clocked: bool) {
        // Counter register effects arrive one CPU cycle after the write, after
        // that cycle's frame clock. Other channel registers update immediately.
        // A simultaneous reload wins only if the counter was already zero.
        if let Some(value) = self.pending_reload.take() {
            if self.enabled && (!clocked || self.before_clock == 0) {
                self.value = LENGTHS[(value >> 3) as usize];
            }
        }
        if let Some(halt) = self.pending_halt.take() {
            self.halt = halt;
        }
    }

    pub fn clock(&mut self) {
        self.before_clock = self.value;
        if !self.halt && self.value > 0 {
            self.value -= 1;
        }
    }
}

#[derive(Default)]
pub(super) struct Envelope {
    pub period: u8,
    pub constant: bool,
    pub looping: bool,
    pub start: bool,
    pub divider: u8,
    pub decay: u8,
}

impl Envelope {
    pub fn write(&mut self, value: u8) {
        self.period = value & 15;
        self.constant = value & 0x10 != 0;
        self.looping = value & 0x20 != 0;
    }

    pub fn clock(&mut self) {
        if self.start {
            self.start = false;
            self.decay = 15;
            self.divider = self.period;
        } else if self.divider == 0 {
            self.divider = self.period;
            if self.decay > 0 {
                self.decay -= 1;
            } else if self.looping {
                self.decay = 15;
            }
        } else {
            self.divider -= 1;
        }
    }

    pub fn output(&self) -> u8 {
        if self.constant {
            self.period
        } else {
            self.decay
        }
    }
}

const DUTIES: [[u8; 8]; 4] = [
    [0, 1, 0, 0, 0, 0, 0, 0],
    [0, 1, 1, 0, 0, 0, 0, 0],
    [0, 1, 1, 1, 1, 0, 0, 0],
    [1, 0, 0, 1, 1, 1, 1, 1],
];

#[derive(Default)]
pub(super) struct Pulse {
    pub length: Length,
    pub envelope: Envelope,
    pub period: u16,
    pub timer: u16,
    pub duty: u8,
    pub position: u8,
    pub sweep_enabled: bool,
    pub sweep_period: u8,
    pub sweep_negate: bool,
    pub sweep_shift: u8,
    pub sweep_reload: bool,
    pub sweep_divider: u8,
    pub first: bool,
}

impl Pulse {
    pub fn write(&mut self, register: u16, value: u8) {
        match register {
            0 => {
                self.duty = value >> 6;
                self.envelope.write(value);
                self.length.set_halt(value & 0x20 != 0);
            }
            1 => {
                self.sweep_enabled = value & 0x80 != 0;
                self.sweep_period = (value >> 4) & 7;
                self.sweep_negate = value & 8 != 0;
                self.sweep_shift = value & 7;
                self.sweep_reload = true;
            }
            2 => self.period = (self.period & 0x700) | value as u16,
            3 => {
                self.period = (self.period & 0xff) | (((value & 7) as u16) << 8);
                self.length.load(value);
                self.position = 0;
                self.envelope.start = true;
            }
            _ => unreachable!(),
        }
    }

    pub fn tick(&mut self) {
        if self.timer == 0 {
            self.timer = self.period;
            self.position = (self.position + 1) & 7;
        } else {
            self.timer -= 1;
        }
    }

    pub fn sweep_target(&self) -> i32 {
        let change = (self.period >> self.sweep_shift) as i32;
        if self.sweep_negate {
            self.period as i32 - change - self.first as i32
        } else {
            self.period as i32 + change
        }
    }

    pub fn muted(&self) -> bool {
        self.period < 8 || self.sweep_target() > 0x7ff
    }

    pub fn clock_sweep(&mut self) {
        if self.sweep_divider == 0 && self.sweep_enabled && self.sweep_shift > 0 && !self.muted() {
            self.period = self.sweep_target() as u16;
        }
        if self.sweep_divider == 0 || self.sweep_reload {
            self.sweep_divider = self.sweep_period;
            self.sweep_reload = false;
        } else {
            self.sweep_divider -= 1;
        }
    }

    pub fn output(&self) -> u8 {
        if self.length.value == 0
            || self.muted()
            || DUTIES[self.duty as usize][self.position as usize] == 0
        {
            0
        } else {
            self.envelope.output()
        }
    }
}

#[derive(Default)]
pub(super) struct Triangle {
    pub length: Length,
    pub period: u16,
    pub timer: u16,
    pub position: u8,
    pub linear_reload: u8,
    pub linear: u8,
    pub reload: bool,
    pub control: bool,
}

impl Triangle {
    pub fn write(&mut self, register: u16, value: u8) {
        match register {
            0 => {
                self.control = value & 0x80 != 0;
                self.length.set_halt(self.control);
                self.linear_reload = value & 0x7f;
            }
            2 => self.period = (self.period & 0x700) | value as u16,
            3 => {
                self.period = (self.period & 0xff) | (((value & 7) as u16) << 8);
                self.length.load(value);
                self.reload = true;
            }
            _ => {} // $4009 is unused.
        }
    }

    pub fn clock_linear(&mut self) {
        if self.reload {
            self.linear = self.linear_reload;
        } else if self.linear > 0 {
            self.linear -= 1;
        }
        if !self.control {
            self.reload = false;
        }
    }

    pub fn tick(&mut self) {
        if self.timer == 0 {
            self.timer = self.period;
            if self.length.value > 0 && self.linear > 0 {
                self.position = (self.position + 1) & 31;
            }
        } else {
            self.timer -= 1;
        }
    }

    pub fn output(&self) -> u8 {
        // The DAC holds its last value when either counter gates the sequencer.
        if self.position < 16 {
            15 - self.position
        } else {
            self.position - 16
        }
    }
}

pub(super) const NOISE_PERIODS: [u16; 16] = [
    4, 8, 16, 32, 64, 96, 128, 160, 202, 254, 380, 508, 762, 1016, 2034, 4068,
];

pub(super) struct Noise {
    pub length: Length,
    pub envelope: Envelope,
    pub timer: u16,
    pub period_index: u8,
    pub short: bool,
    pub shift: u16,
}

impl Noise {
    pub fn new() -> Noise {
        Noise {
            length: Length::default(),
            envelope: Envelope::default(),
            timer: 0,
            period_index: 0,
            short: false,
            shift: 1,
        }
    }

    pub fn write(&mut self, register: u16, value: u8) {
        match register {
            0 => {
                self.envelope.write(value);
                self.length.set_halt(value & 0x20 != 0);
            }
            2 => {
                self.period_index = value & 15;
                self.short = value & 0x80 != 0;
            }
            3 => {
                self.length.load(value);
                self.envelope.start = true;
            }
            _ => {}
        }
    }

    pub fn tick(&mut self) {
        if self.timer == 0 {
            self.timer = NOISE_PERIODS[self.period_index as usize] - 1;
            let tap = if self.short { 6 } else { 1 };
            let feedback = (self.shift ^ (self.shift >> tap)) & 1;
            self.shift = (self.shift >> 1) | (feedback << 14);
        } else {
            self.timer -= 1;
        }
    }

    pub fn output(&self) -> u8 {
        if self.length.value == 0 || self.shift & 1 != 0 {
            0
        } else {
            self.envelope.output()
        }
    }
}
