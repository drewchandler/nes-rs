use cpu::Cpu;
use interconnect::MemoryMappingInterconnect;
use rom::Rom;
use video_driver::VideoDriver;

pub struct Nes {
    pub interconnect: MemoryMappingInterconnect,
    pub cpu: Cpu,
}

impl Nes {
    pub fn new(rom: Rom) -> Nes {
        Nes {
            interconnect: MemoryMappingInterconnect::new(rom),
            cpu: Cpu::new(),
        }
    }

    pub fn reset(&mut self) {
        self.cpu.reset(&mut self.interconnect);
    }

    pub fn run_frame(&mut self, video_driver: &mut Box<VideoDriver>) {
        let mut frame_in_progress = true;

        while frame_in_progress {
            let cycles = self.cpu.step(&mut self.interconnect);

            for _ in 0..cycles {
                let result = self.interconnect.ppu.step();

                if result.nmi {
                    self.cpu.nmi(&mut self.interconnect);
                }

                if result.end_frame {
                    frame_in_progress = false;
                }
            }
        }

        video_driver.output_frame(&self.interconnect.ppu.screen);
    }
}
