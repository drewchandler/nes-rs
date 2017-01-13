use cpu::Cpu;
use interconnect::MemoryMappingInterconnect;
use rom::Rom;

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
        self.cpu.reset(&self.interconnect);
    }

    pub fn run_frame(&mut self) {
        self.cpu.step(&mut self.interconnect);

        let mut vblank_occurred = false;
        for _ in 0..6 {
            vblank_occurred = vblank_occurred || self.interconnect.ppu.step();
        }

        if vblank_occurred {
            self.cpu.nmi(&mut self.interconnect);
        }
    }
}
