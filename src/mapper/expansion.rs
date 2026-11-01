// Clocked cartridge sound units. Their digital outputs join the APU before
// resampling; the relative analog mixing gains remain an approximation.
#[derive(Default)]
pub struct Vrc6Audio {
    regs: [[u8; 3]; 3],
    timer: [u16; 3],
    phase: [u8; 3],
    accumulator: u8,
    control: u8,
}
impl Vrc6Audio {
    pub fn write(&mut self, channel: usize, register: usize, value: u8) {
        self.regs[channel][register] = value;
        if register == 2 && value & 0x80 == 0 {
            self.phase[channel] = 0;
            if channel == 2 {
                self.accumulator = 0;
            }
        }
    }
    pub fn control(&mut self, value: u8) {
        self.control = value & 7;
    }
    pub fn tick(&mut self) {
        if self.control & 1 != 0 {
            return;
        }
        let shift = if self.control & 4 != 0 {
            8
        } else if self.control & 2 != 0 {
            4
        } else {
            0
        };
        for channel in 0..3 {
            if self.regs[channel][2] & 0x80 == 0 {
                continue;
            }
            if self.timer[channel] != 0 {
                self.timer[channel] -= 1;
                continue;
            }
            self.timer[channel] = ((self.regs[channel][1] as u16)
                | (((self.regs[channel][2] & 15) as u16) << 8))
                >> shift;
            if channel < 2 {
                self.phase[channel] = (self.phase[channel] + 1) & 15;
            } else {
                if self.phase[2] & 1 == 0 {
                    self.accumulator = self.accumulator.wrapping_add(self.regs[2][0] & 63);
                }
                self.phase[2] += 1;
                if self.phase[2] == 14 {
                    self.phase[2] = 0;
                    self.accumulator = 0;
                }
            }
        }
    }
    pub fn output(&self) -> f32 {
        let mut sum = 0;
        for ch in 0..2 {
            if self.regs[ch][2] & 0x80 != 0
                && (self.regs[ch][0] & 0x80 != 0 || self.phase[ch] <= ((self.regs[ch][0] >> 4) & 7))
            {
                sum += self.regs[ch][0] & 15;
            }
        }
        if self.regs[2][2] & 0x80 != 0 {
            sum += self.accumulator >> 3;
        }
        sum as f32 * 0.006
    }
}
state_fields!(Vrc6Audio, regs,timer,phase,accumulator,control;|s: &Vrc6Audio| s.phase[0]<16 && s.phase[1]<16 && s.phase[2]<14 && s.control<8);

pub struct Psg {
    regs: [u8; 16],
    select: u8,
    divider: u8,
    timers: [u16; 3],
    high: [bool; 3],
    noise_timer: u8,
    noise: u32,
    envelope_timer: u16,
    envelope_step: u8,
    attack: u8,
    holding: bool,
}
impl Psg {
    pub fn new() -> Self {
        Psg {
            regs: [0; 16],
            select: 0,
            divider: 0,
            timers: [0; 3],
            high: [false; 3],
            noise_timer: 0,
            noise: 1,
            envelope_timer: 0,
            envelope_step: 0,
            attack: 0,
            holding: true,
        }
    }
    pub fn select(&mut self, value: u8) {
        self.select = value & 15;
    }
    pub fn write(&mut self, value: u8) {
        let index = self.select as usize;
        let mask = match index {
            1 | 3 | 5 | 13 => 15,
            6 | 8 | 9 | 10 => 31,
            _ => 255,
        };
        self.regs[index] = value & mask;
        if index == 13 {
            self.envelope_step = 31;
            self.attack = if value & 4 != 0 { 31 } else { 0 };
            self.holding = false;
            self.envelope_timer = 0;
        }
    }
    pub fn tick(&mut self) {
        self.divider = (self.divider + 1) & 31;
        if self.divider & 15 == 0 {
            for ch in 0..3 {
                if self.timers[ch] > 1 {
                    self.timers[ch] -= 1;
                } else {
                    self.timers[ch] =
                        ((self.regs[ch * 2] as u16) | ((self.regs[ch * 2 + 1] as u16) << 8)).max(1);
                    self.high[ch] = !self.high[ch];
                }
            }
            if self.envelope_timer > 1 {
                self.envelope_timer -= 1;
            } else {
                self.envelope_timer =
                    ((self.regs[11] as u16) | ((self.regs[12] as u16) << 8)).max(1);
                if !self.holding {
                    if self.envelope_step > 0 {
                        self.envelope_step -= 1;
                    } else if self.regs[13] & 8 == 0 {
                        self.holding = true;
                        self.attack = 0;
                    } else {
                        if self.regs[13] & 2 != 0 {
                            self.attack ^= 31;
                        }
                        if self.regs[13] & 1 != 0 {
                            self.holding = true;
                        } else {
                            self.envelope_step = 31;
                        }
                    }
                }
            }
        }
        if self.divider == 0 {
            if self.noise_timer > 1 {
                self.noise_timer -= 1;
            } else {
                self.noise_timer = self.regs[6].max(1);
                self.noise = (self.noise >> 1) | (((self.noise ^ (self.noise >> 3)) & 1) << 16);
            }
        }
    }
    pub fn output(&self) -> f32 {
        use std::sync::OnceLock;
        static LEVELS: OnceLock<[f32; 32]> = OnceLock::new();
        let levels = LEVELS.get_or_init(|| {
            ::std::array::from_fn(|i| {
                if i == 0 {
                    0.0
                } else {
                    2f32.powf((i as f32 - 31.0) / 4.0)
                }
            })
        });
        let mut sum = 0.0;
        for ch in 0..3 {
            if (self.high[ch] || self.regs[7] & (1 << ch) != 0)
                && (self.noise & 1 != 0 || self.regs[7] & (8 << ch) != 0)
            {
                let value = self.regs[8 + ch];
                let level = if value & 16 != 0 {
                    self.envelope_step ^ self.attack
                } else if value & 15 == 0 {
                    0
                } else {
                    ((value & 15) << 1) | 1
                };
                sum += levels[level as usize] * 0.12;
            }
        }
        sum
    }
}
state_fields!(Psg, regs,select,divider,timers,high,noise_timer,noise,envelope_timer,envelope_step,attack,holding;|s: &Psg| s.select<16 && s.divider<32 && s.envelope_step<32 && (s.attack==0 || s.attack==31) && s.noise<0x20000);

#[cfg(test)]
mod tests {
    use super::*;
    use state::{Reader, Snapshot, Writer};
    #[test]
    fn vrc6_pulse_duty_period_halt_and_saw_accumulator_follow_registers() {
        let mut audio = Vrc6Audio::default();
        audio.write(0, 0, 0x7f);
        audio.write(0, 1, 7);
        audio.write(0, 2, 0x80);
        let samples: Vec<_> = (0..128)
            .map(|_| {
                audio.tick();
                audio.output()
            })
            .collect();
        assert_eq!(
            samples.iter().filter(|&&s| s > 0.0).count(),
            64,
            "8/16 duty, 8 CPU cycles per step"
        );
        audio.control(1);
        let held = audio.output();
        for _ in 0..100 {
            audio.tick();
            assert_eq!(audio.output(), held);
        }
        audio.write(0, 2, 0);
        assert_eq!(audio.output(), 0.0);
        let mut saw = Vrc6Audio::default();
        saw.write(2, 0, 8);
        saw.write(2, 1, 0);
        saw.write(2, 2, 0x80);
        let levels: Vec<_> = (0..28)
            .map(|_| {
                saw.tick();
                (saw.output() / 0.006).round() as u8
            })
            .collect();
        assert_eq!(&levels[..14], &[1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 0]);
        assert_eq!(levels[..14], levels[14..]);
    }
    fn psg_write(p: &mut Psg, register: u8, value: u8) {
        p.select(register);
        p.write(value);
    }
    #[test]
    fn psg_tone_envelope_noise_and_snapshot_preserve_digital_phase() {
        let mut tone = Psg::new();
        psg_write(&mut tone, 0, 5);
        psg_write(&mut tone, 7, 0x38);
        psg_write(&mut tone, 8, 15);
        let high = (0..160)
            .filter(|_| {
                tone.tick();
                tone.output() > 0.0
            })
            .count();
        assert_eq!(high, 80, "tone half-period = 16 * period CPU cycles");
        let mut env = Psg::new();
        psg_write(&mut env, 7, 0x3f);
        psg_write(&mut env, 8, 16);
        psg_write(&mut env, 11, 1);
        psg_write(&mut env, 13, 8);
        let initial = env.output();
        let mut levels = vec![];
        for _ in 0..32 {
            for _ in 0..16 {
                env.tick();
            }
            levels.push(env.output());
        }
        assert_eq!(levels[30], 0.0);
        assert_eq!(levels[31], initial);
        assert!(levels[..31].windows(2).all(|w| w[0] > w[1]));
        psg_write(&mut env, 13, 15);
        for _ in 0..(16 * 34) {
            env.tick();
        }
        assert_eq!(env.output(), 0.0, "ascending alternate hold ends at zero");
        let mut noise = Psg::new();
        psg_write(&mut noise, 7, 7);
        psg_write(&mut noise, 6, 1);
        psg_write(&mut noise, 8, 15);
        let mut writer = Writer::new();
        noise.save(&mut writer);
        let first: Vec<_> = (0..4096)
            .map(|_| {
                noise.tick();
                noise.output()
            })
            .collect();
        assert!(first.iter().any(|&x| x > 0.0) && first.contains(&0.0));
        noise.load(&mut Reader::new(&writer.0)).unwrap();
        let second: Vec<_> = (0..4096)
            .map(|_| {
                noise.tick();
                noise.output()
            })
            .collect();
        assert_eq!(first, second);
    }
}
