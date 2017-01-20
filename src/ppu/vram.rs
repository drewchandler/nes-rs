pub struct Vram([u8; 16384]);

fn map_addr(addr: u16) -> usize {
    match addr {
        0...0x2fff | 0x3f00...0x3f1f => addr as usize,
        0x3000...0x3eff => addr as usize - 0x1000,
        0x3f20...0x3fff => ((addr - 0x3f20) % 32 + 0x3f00) as usize,
        0x4000...0xffff => map_addr(addr % 0x4000),
        _ => panic!("UNIMPLEMENTED ADDR {:x}", addr),
    }
}

impl Vram {
    pub fn new() -> Vram {
        Vram([0; 16384])
    }

    pub fn read(&self, addr: u16) -> u8 {
        self.0[map_addr(addr)]
    }

    pub fn write(&mut self, addr: u16, value: u8) {
        self.0[map_addr(addr)] = value;
    }
}
