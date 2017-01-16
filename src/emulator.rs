use nes::Nes;
use rom::Rom;
use video_driver::{VideoDriver, MiniFBWindow};

pub struct Emulator {
    nes: Nes,
    video_driver: Box<VideoDriver>,
}

impl Emulator {
    pub fn new(rom: Rom) -> Emulator {
        Emulator {
            nes: Nes::new(rom),
            video_driver: Box::new(MiniFBWindow::new()),
        }
    }

    pub fn run(&mut self) {
        self.nes.reset();

        while self.video_driver.is_open() {
            self.nes.run_frame(&mut self.video_driver);
        }
    }
}
