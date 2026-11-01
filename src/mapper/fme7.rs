use mapper::{expansion::Psg, memory::Memory, Mapper};
use rom::{Mirroring, Rom};
pub struct Fme7 {
    mem: Memory,
    select: u8,
    chr: [u8; 8],
    prg: [u8; 4],
    mirror: Mirroring,
    irq_control: u8,
    counter: u16,
    pending: bool,
    audio: Psg,
}
impl Fme7 {
    pub fn new(rom: Rom) -> Self {
        let mirror = rom.mirroring;
        Fme7 {
            mem: Memory::new(rom),
            select: 0,
            chr: [0, 1, 2, 3, 4, 5, 6, 7],
            prg: [0; 4],
            mirror,
            irq_control: 0,
            counter: 0,
            pending: false,
            audio: Psg::new(),
        }
    }
}
impl Mapper for Fme7 {
    fn memory(&self) -> Option<&Memory> {
        Some(&self.mem)
    }
    fn memory_mut(&mut self) -> Option<&mut Memory> {
        Some(&mut self.mem)
    }
    fn read(&self, addr: u16) -> u8 {
        let slot = (addr - 0x8000) as usize / 8192;
        self.mem.prg(
            addr,
            8192,
            if slot == 3 {
                self.mem.prg.len() / 8192 - 1
            } else {
                (self.prg[slot + 1] & 63) as usize
            },
        )
    }
    fn write(&mut self, addr: u16, value: u8) {
        match addr & 0xe000 {
            0x8000 => self.select = value & 15,
            0xa000 => match self.select {
                0..=7 => self.chr[self.select as usize] = value,
                8..=11 => self.prg[(self.select - 8) as usize] = value,
                12 => {
                    self.mirror = match value & 3 {
                        0 => Mirroring::Vertical,
                        1 => Mirroring::Horizontal,
                        2 => Mirroring::SingleLower,
                        _ => Mirroring::SingleUpper,
                    }
                }
                13 => {
                    self.irq_control = value;
                    self.pending = false;
                }
                14 => self.counter = (self.counter & 0xff00) | value as u16,
                15 => self.counter = (self.counter & 255) | ((value as u16) << 8),
                _ => {}
            },
            0xc000 => self.audio.select(value),
            0xe000 => self.audio.write(value),
            _ => {}
        }
    }
    fn read_low(&self, addr: u16) -> Option<u8> {
        if addr < 0x6000 {
            return None;
        }
        if self.prg[0] & 0x40 != 0 {
            if self.prg[0] & 0x80 != 0 {
                self.mem.read_ram(addr)
            } else {
                None
            }
        } else {
            Some(self.mem.prg(addr, 8192, (self.prg[0] & 63) as usize))
        }
    }
    fn write_low(&mut self, addr: u16, value: u8) {
        if self.prg[0] & 0xc0 == 0xc0 {
            self.mem.write_ram(addr, value);
        }
    }
    fn mirroring(&self) -> Option<Mirroring> {
        Some(self.mirror)
    }
    fn read_chr(&self, addr: u16) -> Option<u8> {
        Some(self.mem.chr.read(Memory::chr_addr(
            addr,
            1024,
            self.chr[addr as usize / 1024] as usize,
        )))
    }
    fn write_chr(&mut self, addr: u16, value: u8) {
        let offset = Memory::chr_addr(addr, 1024, self.chr[addr as usize / 1024] as usize);
        self.mem.chr.write(offset, value);
    }
    fn clock_cpu(&mut self) {
        self.audio.tick();
        if self.irq_control & 0x80 != 0 {
            let before = self.counter;
            self.counter = self.counter.wrapping_sub(1);
            if before == 0 && self.irq_control & 1 != 0 {
                self.pending = true;
            }
        }
    }
    fn audio(&self) -> f32 {
        self.audio.output()
    }
    fn irq(&self) -> bool {
        self.pending
    }
}
state_fields!(Fme7,mem,select,chr,prg,mirror,irq_control,counter,pending,audio;|s: &Fme7|s.select<16);
