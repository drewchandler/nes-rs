use blip_buf::BlipBuf;
use std::collections::VecDeque;
use std::sync::OnceLock;

pub const SAMPLE_RATE: u32 = 48_000;
pub const CPU_HZ: f64 = 21_477_272.727272727 / 12.0;
const BLOCK_CYCLES: u32 = 20_000;
const MAX_SAMPLES: usize = SAMPLE_RATE as usize / 10;

// Independent pulse and TND circuits have nonlinear responses. Cache the exact
// transfer curves rather than adding the five DAC outputs linearly.
pub(super) fn mix(p1: u8, p2: u8, triangle: u8, noise: u8, dmc: u8) -> f32 {
    static TABLES: OnceLock<(Vec<f32>, Vec<f32>)> = OnceLock::new();
    let tables = TABLES.get_or_init(|| {
        let pulse = (0..31)
            .map(|p| {
                if p == 0 {
                    0.0
                } else {
                    95.88 / (8128.0 / p as f32 + 100.0)
                }
            })
            .collect();
        let tnd = (0..16 * 16 * 128)
            .map(|index| {
                let t = (index >> 11) as f32 / 8227.0;
                let n = ((index >> 7) & 15) as f32 / 12241.0;
                let d = (index & 127) as f32 / 22638.0;
                if index == 0 {
                    0.0
                } else {
                    159.79 / (1.0 / (t + n + d) + 100.0)
                }
            })
            .collect();
        (pulse, tnd)
    });
    tables.0[(p1 + p2) as usize]
        + tables.1[((triangle as usize) << 11) | ((noise as usize) << 7) | dmc as usize]
}

struct HighPass {
    coefficient: f32,
    previous_in: f32,
    previous_out: f32,
}

impl HighPass {
    fn new(hz: f32) -> HighPass {
        HighPass {
            coefficient: (-2.0 * ::std::f32::consts::PI * hz / SAMPLE_RATE as f32).exp(),
            previous_in: 0.0,
            previous_out: 0.0,
        }
    }

    fn apply(&mut self, value: f32) -> f32 {
        let output = self.coefficient * (self.previous_out + value - self.previous_in);
        self.previous_in = value;
        self.previous_out = output;
        output
    }
}

pub(super) struct Mixer {
    blip: BlipBuf,
    clock: u32,
    previous: i32,
    samples: VecDeque<i16>,
    high90: HighPass,
    high440: HighPass,
    low_coefficient: f32,
    low_previous: f32,
}

impl Mixer {
    pub fn new() -> Mixer {
        Self::with_rate(CPU_HZ)
    }
    pub fn with_rate(cpu_hz: f64) -> Mixer {
        let mut blip = BlipBuf::new(2048);
        blip.set_rates(cpu_hz, SAMPLE_RATE as f64);
        Mixer {
            blip,
            clock: 0,
            previous: 0,
            samples: VecDeque::new(),
            high90: HighPass::new(90.0),
            high440: HighPass::new(440.0),
            low_coefficient: 1.0
                - (-2.0 * ::std::f32::consts::PI * 14000.0 / SAMPLE_RATE as f32).exp(),
            low_previous: 0.0,
        }
    }

    pub fn tick(&mut self, output: f32) {
        let level = (output * 24_000.0) as i32;
        if level != self.previous {
            self.blip.add_delta(self.clock, level - self.previous);
            self.previous = level;
        }
        self.clock += 1;
        if self.clock == BLOCK_CYCLES {
            self.flush();
        }
    }

    fn flush(&mut self) {
        self.blip.end_frame(self.clock);
        self.clock = 0;
        let mut block = [0i16; 1024];
        let count = self.blip.read_samples(&mut block, false);
        for &sample in &block[..count] {
            let high = self.high90.apply(sample as f32);
            let high = self.high440.apply(high);
            self.low_previous += self.low_coefficient * (high - self.low_previous);
            let filtered = self.low_previous.max(i16::MIN as f32).min(i16::MAX as f32) as i16;
            // Headless callers may never consume audio. Keep memory bounded.
            if self.samples.len() == MAX_SAMPLES {
                self.samples.pop_front();
            }
            self.samples.push_back(filtered);
        }
    }

    pub fn take_samples(&mut self) -> Vec<i16> {
        self.flush();
        self.samples.drain(..).collect()
    }
}
