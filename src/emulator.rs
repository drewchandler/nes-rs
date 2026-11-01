use audio::{AudioOutput, FramePacer};
use controls::NativeControls;
use minifb::{Key, KeyRepeat, Window, WindowOptions};
use nes::Nes;
use persistence::atomic_write;
use rom::Rom;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

pub struct Emulator {
    nes: Nes,
    window: Window,
    audio: Option<AudioOutput>,
    controls: NativeControls,
    save_path: PathBuf,
    state_path: PathBuf,
    last_save: Vec<u8>,
    save_writable: bool,
}
impl Emulator {
    pub fn new(rom: Rom, path: &Path, controls: Option<&Path>) -> Emulator {
        let audio = match AudioOutput::new() {
            Ok(a) => Some(a),
            Err(e) => {
                eprintln!("Audio unavailable: {}", e);
                None
            }
        };
        let mut nes = Nes::new(rom);
        let save_path = path.with_extension("sav");
        let mut save_writable = true;
        if !nes.battery().is_empty() {
            match ::std::fs::read(&save_path) {
                Ok(bytes) => {
                    if let Err(e) = nes.load_battery(&bytes) {
                        eprintln!("Save not loaded: {}. The existing file will be kept.", e);
                        save_writable = false;
                    }
                }
                Err(ref e) if e.kind() == ::std::io::ErrorKind::NotFound => {}
                Err(e) => {
                    eprintln!("Save not loaded: {}. The existing file will be kept.", e);
                    save_writable = false;
                }
            }
        }
        let last_save = nes.battery();
        Emulator {
            nes,
            window: Window::new(
                "NES · F5 save state · F8 load state · Esc pauses",
                256,
                240,
                WindowOptions::default(),
            )
            .unwrap(),
            audio,
            controls: NativeControls::new(controls),
            save_path,
            state_path: path.with_extension("nss"),
            last_save,
            save_writable,
        }
    }
    fn save_battery(&mut self) {
        if !self.save_writable {
            return;
        }
        let bytes = self.nes.battery();
        if bytes.is_empty() || bytes == self.last_save {
            return;
        }
        match atomic_write(&self.save_path, &bytes) {
            Ok(()) => self.last_save = bytes,
            Err(e) => {
                eprintln!("Could not save {}: {}", self.save_path.display(), e);
                self.window
                    .set_title("NES · Game save failed; check directory permissions");
            }
        }
    }
    pub fn run(&mut self) {
        self.nes.reset();
        let mut pacer = FramePacer::with_fps(self.nes.fps());
        let mut last_check = Instant::now();
        let mut paused = false;
        while self.window.is_open() {
            if self.window.is_key_pressed(Key::Escape, KeyRepeat::No) {
                paused = !paused;
                if let Some(ref audio) = self.audio {
                    audio.clear();
                }
            }
            if self.window.is_key_pressed(Key::F5, KeyRepeat::No) {
                match atomic_write(&self.state_path, &self.nes.save_state()) {
                    Ok(()) => self.window.set_title("NES · State saved · F8 restores"),
                    Err(e) => {
                        eprintln!("State save failed: {}", e);
                        self.window.set_title("NES · State save failed");
                    }
                }
            }
            if self.window.is_key_pressed(Key::F8, KeyRepeat::No) {
                let result = ::std::fs::read(&self.state_path)
                    .map_err(|e| e.to_string())
                    .and_then(|b| self.nes.load_state(&b));
                match result {
                    Ok(()) => {
                        if let Some(ref audio) = self.audio {
                            audio.clear();
                        }
                        self.window.set_title("NES · State restored");
                        self.save_battery();
                    }
                    Err(e) => {
                        eprintln!("State load failed: {}", e);
                        self.window
                            .set_title("NES · State load failed; current game kept");
                    }
                }
                pacer = FramePacer::with_fps(self.nes.fps());
            }
            let pads = if self.window.is_active() {
                self.controls.read(&self.window)
            } else {
                [::joypad::ButtonState::default(); 2]
            };
            if paused {
                self.window
                    .update_with_buffer(&self.nes.interconnect.ppu.screen);
                pacer.wait();
            } else {
                let frame = self.nes.run_frame_two(pads[0], pads[1]);
                self.window.update_with_buffer(frame);
                let samples = self.nes.interconnect.apu.take_samples();
                let error = self.audio.as_mut().and_then(|a| a.submit(&samples).err());
                if let Some(e) = error {
                    eprintln!("Audio unavailable: {}", e);
                    self.audio = None;
                    pacer = FramePacer::with_fps(self.nes.fps());
                }
                if self.audio.is_none() {
                    pacer.wait();
                }
            }
            if last_check.elapsed() >= Duration::from_secs(2) {
                self.save_battery();
                last_check = Instant::now();
            }
        }
        self.save_battery();
    }
}
