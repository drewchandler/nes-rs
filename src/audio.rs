use apu::{CPU_HZ, SAMPLE_RATE};
use sdl2::audio::{AudioQueue, AudioSpecDesired};
use sdl2::AudioSubsystem;
use sdl2::Sdl;
use std::thread;
use std::time::{Duration, Instant};

pub struct AudioOutput {
    queue: AudioQueue<i16>,
    _audio: AudioSubsystem,
    _sdl: Sdl,
    started: bool,
}

impl AudioOutput {
    pub fn new() -> Result<AudioOutput, String> {
        // minifb owns the event loop. SDL must not turn SIGINT/SIGTERM into
        // unhandled SDL events merely because we initialized its audio backend.
        ::sdl2::hint::set("SDL_NO_SIGNAL_HANDLERS", "1");
        let sdl = ::sdl2::init()?;
        let audio = sdl.audio()?;
        let desired = AudioSpecDesired {
            freq: Some(SAMPLE_RATE as i32),
            channels: Some(1),
            samples: Some(1024),
        };
        let queue = audio.open_queue(None, &desired)?;
        Ok(AudioOutput {
            queue,
            _audio: audio,
            _sdl: sdl,
            started: false,
        })
    }

    pub fn submit(&mut self, samples: &[i16]) -> Result<(), String> {
        self.queue.queue_audio(samples)?;
        if !self.started && !samples.is_empty() {
            self.queue.resume();
            self.started = true;
        }
        // Let the device pace emulation, with about two frames of headroom.
        // A stalled/unplugged device must not freeze input or grow the queue.
        let start = Instant::now();
        let target_bytes = SAMPLE_RATE / 30 * 2;
        while self.queue.size() > target_bytes {
            if start.elapsed() > Duration::from_millis(250) {
                return Err("audio device stopped consuming samples".to_string());
            }
            thread::sleep(Duration::from_millis(2));
        }
        Ok(())
    }
}

pub struct FramePacer {
    next: Instant,
    period: Duration,
}

impl FramePacer {
    pub fn new() -> FramePacer {
        // NTSC alternates 89342/89341 PPU dots with rendering enabled.
        let seconds = (341.0 * 262.0 - 0.5) / (CPU_HZ * 3.0);
        FramePacer {
            next: Instant::now(),
            period: Duration::from_secs_f64(seconds),
        }
    }

    pub fn wait(&mut self) {
        self.next += self.period;
        let now = Instant::now();
        if self.next > now {
            thread::sleep(self.next - now);
        } else if now.duration_since(self.next) > self.period * 4 {
            self.next = now;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "Opens an audio device; use SDL_AUDIODRIVER=dummy for headless validation"]
    fn queued_audio_is_consumed_and_remains_bounded() {
        let mut output = AudioOutput::new().unwrap();
        let samples: Vec<i16> = (0..800)
            .map(|i| if i % 100 < 50 { 4000 } else { -4000 })
            .collect();
        let start = Instant::now();
        for _ in 0..12 {
            output.submit(&samples).unwrap();
            assert!(output.queue.size() <= SAMPLE_RATE / 30 * 2);
        }
        assert!(start.elapsed() > Duration::from_millis(100));
        assert_eq!(output.queue.spec().freq, SAMPLE_RATE as i32);
        assert_eq!(output.queue.spec().channels, 1);
    }
}
