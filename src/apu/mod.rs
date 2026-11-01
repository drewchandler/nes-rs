mod channels;
mod dmc;
mod mixer;

use self::channels::{Noise, Pulse, Triangle};
use self::dmc::Dmc;
use self::mixer::{mix, Mixer};
pub use self::mixer::{CPU_HZ, SAMPLE_RATE};

pub struct Apu {
    pal: bool,
    pulse1: Pulse,
    pulse2: Pulse,
    triangle: Triangle,
    noise: Noise,
    dmc: Dmc,
    cycle: u64,
    frame_cycle: u32,
    mode5: bool,
    next_mode5: bool,
    inhibit_irq: bool,
    frame_irq: bool,
    reset_delay: u8,
    length_clocked: bool,
    mixer: Mixer,
}

impl Apu {
    pub fn new() -> Apu {
        let mut pulse1 = Pulse::default();
        pulse1.first = true;
        let mut triangle = Triangle::default();
        triangle.position = 15; // Power-up DAC value zero.
        Apu {
            pal: false,
            pulse1,
            pulse2: Pulse::default(),
            triangle,
            noise: Noise::new(),
            dmc: Dmc::new(),
            cycle: 0,
            frame_cycle: 0,
            mode5: false,
            next_mode5: false,
            inhibit_irq: false,
            frame_irq: false,
            reset_delay: 0,
            length_clocked: false,
            mixer: Mixer::new(),
        }
    }

    pub fn with_region(region: ::region::Region) -> Apu {
        let mut apu = Self::new();
        apu.pal = region == ::region::Region::Pal;
        apu.noise.pal = apu.pal;
        apu.dmc.pal = apu.pal;
        apu.mixer = Mixer::with_rate(region.cpu_hz());
        apu
    }

    pub fn reset(&mut self) {
        self.write_register(0x4015, 0);
        self.frame_irq = false;
        // Reset the sequencer after its effective $4017 write delay. The CPU's
        // seven reset cycles then put the first opcode about ten cycles later.
        self.frame_cycle = 0;
        self.mode5 = self.next_mode5;
        self.reset_delay = 0;
        if self.mode5 {
            self.quarter_frame();
            self.half_frame();
        }
    }

    pub fn irq(&self) -> bool {
        self.frame_irq || self.dmc.irq
    }
    pub fn dmc_request(&self) -> Option<u16> {
        self.dmc.request()
    }
    pub fn complete_dmc_dma(&mut self, value: u8) {
        self.dmc.complete_dma(value);
    }
    pub fn take_samples(&mut self) -> Vec<i16> {
        self.mixer.take_samples()
    }

    pub fn read_status(&mut self, open_bus: u8) -> u8 {
        let result = ((self.pulse1.length.value > 0) as u8)
            | (((self.pulse2.length.value > 0) as u8) << 1)
            | (((self.triangle.length.value > 0) as u8) << 2)
            | (((self.noise.length.value > 0) as u8) << 3)
            | (((self.dmc.remaining > 0) as u8) << 4)
            | (open_bus & 0x20)
            | ((self.frame_irq as u8) << 6)
            | ((self.dmc.irq as u8) << 7);
        self.frame_irq = false;
        result
    }

    pub fn write_register(&mut self, addr: u16, value: u8) {
        match addr {
            0x4000..=0x4003 => self.pulse1.write(addr & 3, value),
            0x4004..=0x4007 => self.pulse2.write(addr & 3, value),
            0x4008..=0x400b => self.triangle.write(addr & 3, value),
            0x400c..=0x400f => self.noise.write(addr & 3, value),
            0x4010..=0x4013 => self.dmc.write(addr & 3, value),
            0x4015 => {
                self.pulse1.length.enable(value & 1 != 0);
                self.pulse2.length.enable(value & 2 != 0);
                self.triangle.length.enable(value & 4 != 0);
                self.noise.length.enable(value & 8 != 0);
                self.dmc.enable(value & 16 != 0, self.cycle);
            }
            0x4017 => {
                self.next_mode5 = value & 0x80 != 0;
                self.inhibit_irq = value & 0x40 != 0;
                if self.inhibit_irq {
                    self.frame_irq = false;
                }
                self.reset_delay = if self.cycle & 1 == 0 { 3 } else { 4 };
            }
            _ => {}
        }
    }

    fn quarter_frame(&mut self) {
        self.pulse1.envelope.clock();
        self.pulse2.envelope.clock();
        self.noise.envelope.clock();
        self.triangle.clock_linear();
    }

    fn half_frame(&mut self) {
        self.length_clocked = true;
        self.pulse1.length.clock();
        self.pulse2.length.clock();
        self.triangle.length.clock();
        self.noise.length.clock();
        self.pulse1.clock_sweep();
        self.pulse2.clock_sweep();
    }

    pub fn step(&mut self) {
        self.step_with_audio(0.0);
    }
    pub fn step_with_audio(&mut self, expansion: f32) {
        self.cycle = self.cycle.wrapping_add(1);
        self.length_clocked = false;
        self.frame_cycle += 1;
        let [q1, h1, q2, irq1, h2, irq3, h5, end5] = if self.pal {
            [8313, 16627, 24939, 33252, 33253, 33254, 41565, 41566]
        } else {
            [7457, 14913, 22371, 29828, 29829, 29830, 37281, 37282]
        };
        if self.frame_cycle == q1 || self.frame_cycle == q2 {
            self.quarter_frame();
        }
        if self.frame_cycle == h1
            || (self.mode5 && self.frame_cycle == h5)
            || (!self.mode5 && self.frame_cycle == h2)
        {
            self.quarter_frame();
            self.half_frame();
        }
        if !self.mode5 && self.frame_cycle >= irq1 && self.frame_cycle <= irq3 && !self.inhibit_irq
        {
            self.frame_irq = true;
        }
        if (!self.mode5 && self.frame_cycle == irq3) || (self.mode5 && self.frame_cycle == end5) {
            self.frame_cycle = 0;
        }
        if self.reset_delay > 0 {
            self.reset_delay -= 1;
            if self.reset_delay == 0 {
                self.mode5 = self.next_mode5;
                self.frame_cycle = 0;
                if self.mode5 {
                    self.quarter_frame();
                    self.half_frame();
                }
            }
        }
        self.pulse1.length.commit(self.length_clocked);
        self.pulse2.length.commit(self.length_clocked);
        self.triangle.length.commit(self.length_clocked);
        self.noise.length.commit(self.length_clocked);
        if self.cycle & 1 == 0 {
            self.pulse1.tick();
            self.pulse2.tick();
        }
        self.triangle.tick();
        self.noise.tick();
        self.dmc.tick();
        self.mixer.tick(
            mix(
                self.pulse1.output(),
                self.pulse2.output(),
                self.triangle.output(),
                self.noise.output(),
                self.dmc.output,
            ) + expansion,
        );
    }
}

#[cfg(test)]
mod rom_tests;
#[cfg(test)]
mod tests;

state_fields!(Apu, pulse1,pulse2,triangle,noise,dmc,cycle,frame_cycle,mode5,next_mode5,inhibit_irq,frame_irq,reset_delay,length_clocked;|s: &Apu| s.frame_cycle<(if s.pal {if s.mode5 {41566}else{33254}}else if s.mode5 {37282}else{29830}) && s.reset_delay<=4);
