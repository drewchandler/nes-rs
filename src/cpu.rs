use interconnect::Interconnect;

pub struct Cpu {
    pc: u16,
}

impl Cpu {
    pub fn new() -> Cpu {
        Cpu { pc: 0 }
    }

    pub fn step(&self, interconnect: &Interconnect) {
        println!("{:x}", interconnect.read_word(self.pc));
    }

    pub fn reset(&mut self, interconnect: &Interconnect) {
        self.pc = interconnect.read_double(0xfffc);
    }
}
