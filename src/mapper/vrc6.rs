use mapper::{expansion::Vrc6Audio, memory::Memory, Mapper};
use rom::Rom;
pub struct Vrc6 {
    mem: Memory,
    swapped: bool,
    prg16: u8,
    prg8: u8,
    chr: [u8; 8],
    control: u8,
    latch: u8,
    counter: u8,
    irq_control: u8,
    prescaler: i16,
    pending: bool,
    audio: Vrc6Audio,
}
impl Vrc6 {
    pub fn new(rom: Rom) -> Self {
        let swapped = rom.mapper == 26;
        Vrc6 {
            mem: Memory::new(rom),
            swapped,
            prg16: 0,
            prg8: 0,
            chr: [0, 1, 2, 3, 4, 5, 6, 7],
            control: 0x20,
            latch: 0,
            counter: 0,
            irq_control: 0,
            prescaler: 341,
            pending: false,
            audio: Vrc6Audio::default(),
        }
    }
    fn chr_offset(&self, addr: u16) -> usize {
        let slot = addr as usize / 1024;
        let mode = self.control & 3;
        let reg = if mode == 0 {
            slot
        } else if mode == 1 {
            slot / 2
        } else if slot < 4 {
            slot
        } else {
            4 + (slot - 4) / 2
        };
        let mut bank = self.chr[reg] as usize;
        if self.control & 0x20 != 0 && (mode == 1 || (mode >= 2 && slot >= 4)) {
            bank = (bank & !1) | (slot & 1);
        }
        Memory::chr_addr(addr, 1024, bank)
    }
    fn nt_bank(&self, table: usize) -> usize {
        let index = match self.control & 7 {
            0 | 6 | 7 => 6 + table / 2,
            1 | 5 => 4 + table,
            _ => 6 + (table & 1),
        };
        let mut bank = self.chr[index] as usize;
        if self.control & 0x20 != 0 {
            let forced = match self.control & 15 {
                0 | 7 => Some(table & 1),
                3 | 4 => Some(table / 2),
                8 | 15 => Some(0),
                11 | 12 => Some(1),
                _ => None,
            };
            if let Some(bit) = forced {
                bank = (bank & !1) | bit;
            }
        }
        bank
    }
    fn clock_irq(&mut self) {
        if self.counter == 255 {
            self.counter = self.latch;
            self.pending = true;
        } else {
            self.counter += 1;
        }
    }
}
impl Mapper for Vrc6 {
    fn memory(&self) -> Option<&Memory> {
        Some(&self.mem)
    }
    fn memory_mut(&mut self) -> Option<&mut Memory> {
        Some(&mut self.mem)
    }
    fn read(&self, addr: u16) -> u8 {
        if addr < 0xc000 {
            self.mem.prg(addr, 16384, self.prg16 as usize)
        } else {
            self.mem.prg(
                addr,
                8192,
                if addr < 0xe000 {
                    self.prg8 as usize
                } else {
                    self.mem.prg.len() / 8192 - 1
                },
            )
        }
    }
    fn write(&mut self, addr: u16, value: u8) {
        let low = if self.swapped {
            ((addr & 1) << 1) | ((addr & 2) >> 1)
        } else {
            addr & 3
        };
        let addr = (addr & 0xf000) | low;
        match addr {
            0x8000..=0x8003 => self.prg16 = value & 15,
            0x9000..=0x9002 => self.audio.write(0, low as usize, value),
            0x9003 => self.audio.control(value),
            0xa000..=0xa002 => self.audio.write(1, low as usize, value),
            0xb000..=0xb002 => self.audio.write(2, low as usize, value),
            0xb003 => self.control = value,
            0xc000..=0xc003 => self.prg8 = value & 31,
            0xd000..=0xd003 => self.chr[low as usize] = value,
            0xe000..=0xe003 => self.chr[4 + low as usize] = value,
            0xf000 => self.latch = value,
            0xf001 => {
                self.pending = false;
                self.irq_control = value & 7;
                if value & 2 != 0 {
                    self.counter = self.latch;
                    self.prescaler = 341;
                }
            }
            0xf002 => {
                self.pending = false;
                self.irq_control = (self.irq_control & !2) | ((self.irq_control & 1) << 1);
            }
            _ => {}
        }
    }
    fn read_low(&self, addr: u16) -> Option<u8> {
        if self.control & 0x80 != 0 {
            self.mem.read_ram(addr)
        } else {
            None
        }
    }
    fn write_low(&mut self, addr: u16, value: u8) {
        if self.control & 0x80 != 0 {
            self.mem.write_ram(addr, value);
        }
    }
    fn read_chr(&self, addr: u16) -> Option<u8> {
        Some(self.mem.chr.read(self.chr_offset(addr)))
    }
    fn write_chr(&mut self, addr: u16, value: u8) {
        let offset = self.chr_offset(addr);
        self.mem.chr.write(offset, value);
    }
    fn ciram_page(&self, table: usize) -> Option<usize> {
        Some(self.nt_bank(table) & 1)
    }
    fn nametable_read(&self, addr: u16) -> Option<u8> {
        if self.control & 0x10 != 0 {
            Some(self.mem.chr.read(
                self.nt_bank(((addr & 0xfff) / 1024) as usize) * 1024 + (addr as usize & 1023),
            ))
        } else {
            None
        }
    }
    fn nametable_write(&mut self, addr: u16, value: u8) -> bool {
        if self.control & 0x10 == 0 {
            return false;
        }
        let offset = self.nt_bank(((addr & 0xfff) / 1024) as usize) * 1024 + (addr as usize & 1023);
        self.mem.chr.write(offset, value);
        true
    }
    fn clock_cpu(&mut self) {
        self.audio.tick();
        if self.irq_control & 2 == 0 {
            return;
        }
        if self.irq_control & 4 != 0 {
            self.clock_irq();
        } else {
            self.prescaler -= 3;
            if self.prescaler <= 0 {
                self.prescaler += 341;
                self.clock_irq();
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
state_fields!(Vrc6, mem,prg16,prg8,chr,control,latch,counter,irq_control,prescaler,pending,audio;|s: &Vrc6|s.prg16<16 && s.prg8<32 && s.irq_control<8 && s.prescaler>0 && s.prescaler<=341);
