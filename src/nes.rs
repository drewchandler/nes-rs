use cpu::Cpu;
use interconnect::MemoryMappingInterconnect;
use joypad::ButtonState;
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
        self.cpu.reset(&mut self.interconnect);
    }

    pub fn run_frame(&mut self, joypad1_state: ButtonState) -> &[u32; 256 * 240] {
        self.interconnect.joypad1.set_state(joypad1_state);

        while !self.interconnect.take_frame_ready() {
            self.cpu.step(&mut self.interconnect);
        }

        &self.interconnect.ppu.screen
    }
}
