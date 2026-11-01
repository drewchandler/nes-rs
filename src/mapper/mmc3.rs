use mapper::{Mapper, memory::Memory};
use rom::{Mirroring, Rom};

pub struct Mmc3 {
    mem: Memory,
    dxrom: bool,
    select: u8,
    banks: [u8; 8],
    ram_control: u8,
    mirroring: Mirroring,
    latch: u8,
    counter: u8,
    reload: bool,
    enabled: bool,
    pending: bool,
    a12: bool,
    low_since: u64,
}
impl Mmc3 {
    pub fn new(rom: Rom) -> Mmc3 {
        let dxrom = rom.mapper == 206;
        let mirroring = rom.mirroring;
        Mmc3 { mem: Memory::new(rom), dxrom, select: 0, banks: [0, 2, 4, 5, 6, 7, 0, 1],
            ram_control: 0x80, mirroring, latch: 0, counter: 0, reload: false,
            enabled: false, pending: false, a12: false, low_since: 0 }
    }
    fn chr_addr(&self, addr: u16) -> usize {
        let addr = addr ^ if self.select & 0x80 != 0 { 0x1000 } else { 0 };
        let slot = addr as usize / 1024;
        let bank = match slot {
            0 | 1 => (self.banks[0] & 0xfe) as usize + slot,
            2 | 3 => (self.banks[1] & 0xfe) as usize + slot - 2,
            _ => self.banks[slot - 2] as usize,
        };
        Memory::chr_addr(addr, 1024, bank)
    }
    fn clock_irq(&mut self) {
        let previous = self.counter;
        if self.counter == 0 || self.reload { self.counter = self.latch; }
        else { self.counter -= 1; }
        let old_revision = self.mem.submapper == 4;
        if self.counter == 0 && self.enabled && (!old_revision || previous != 0 || self.reload) {
            // NEC chips suppress automatic 0-to-0 reload interrupts, but
            // an explicit C001 reload can still assert the IRQ.
            self.pending = true;
        }
        self.reload = false;
    }
}
impl Mapper for Mmc3 {
    fn memory(&self) -> Option<&Memory> { Some(&self.mem) }
    fn memory_mut(&mut self) -> Option<&mut Memory> { Some(&mut self.mem) }
    fn read(&self, addr: u16) -> u8 {
        let last = self.mem.prg.len() / 8192 - 1;
        let swap = self.select & 0x40 != 0;
        let bank = match (addr - 0x8000) / 8192 {
            0 => if swap { last - 1 } else { self.banks[6] as usize },
            1 => self.banks[7] as usize,
            2 => if swap { self.banks[6] as usize } else { last - 1 },
            _ => last,
        };
        self.mem.prg(addr, 8192, bank)
    }
    fn write(&mut self, addr: u16, value: u8) {
        if self.dxrom && addr >= 0xa000 { return; }
        match addr & 0xe001 {
            0x8000 => {
                self.select = if self.dxrom { value & 7 } else { value };
                if self.mem.submapper == 1 && value & 0x20 == 0 { self.ram_control = 0; }
            },
            0x8001 => {
                let index = (self.select & 7) as usize;
                self.banks[index] = value & if index >= 6 {
                    if self.dxrom { 15 } else { 63 }
                } else if self.dxrom { 63 } else { 255 };
            }
            0xa000 => if self.mem.mirroring != Mirroring::FourScreen && self.mem.submapper != 2 {
                self.mirroring = if value & 1 == 0 { Mirroring::Vertical } else { Mirroring::Horizontal };
            },
            0xa001 => if self.mem.submapper != 1 || self.select & 0x20 != 0 {
                self.ram_control = value;
            },
            0xc000 => self.latch = value,
            0xc001 => self.reload = true,
            0xe000 => { self.enabled = false; self.pending = false; }
            0xe001 => self.enabled = true,
            _ => {}
        }
    }
    fn read_low(&self, addr: u16) -> Option<u8> {
        if self.mem.submapper == 1 && !self.dxrom {
            if addr < 0x7000 || self.mem.ram.is_empty() || self.select & 0x20 == 0 { return None; }
            let offset = (addr as usize) & 1023;
            let enable = if offset < 512 { 0x20 } else { 0x80 };
            return if self.ram_control & enable != 0 { Some(self.mem.ram[offset]) }
                else if self.ram_control & 0xa0 != 0 { Some(0) } else { None };
        }
        if !self.dxrom && self.ram_control & 0x80 != 0 { self.mem.read_ram(addr) } else { None }
    }
    fn write_low(&mut self, addr: u16, value: u8) {
        if self.mem.submapper == 1 && !self.dxrom {
            if addr < 0x7000 || self.mem.ram.is_empty() || self.select & 0x20 == 0 { return; }
            let offset = (addr as usize) & 1023;
            let enable = if offset < 512 { 0x10 } else { 0x40 };
            if self.ram_control & (enable | enable << 1) == (enable | enable << 1) {
                self.mem.ram[offset] = value;
            }
            return;
        }
        if !self.dxrom && self.ram_control & 0xc0 == 0x80 { self.mem.write_ram(addr, value); }
    }
    fn mirroring(&self) -> Option<Mirroring> { Some(self.mirroring) }
    fn ppu_address(&mut self, addr: u16, cycle: u64) {
        if self.dxrom { return; }
        let high = addr & 0x1000 != 0;
        if !high && self.a12 { self.low_since = cycle; }
        if high && !self.a12 && cycle.wrapping_sub(self.low_since) >= 8 { self.clock_irq(); }
        self.a12 = high;
    }
    fn irq(&self) -> bool { self.pending }
    fn read_chr(&self, addr: u16) -> Option<u8> { Some(self.mem.chr.read(self.chr_addr(addr))) }
    fn write_chr(&mut self, addr: u16, value: u8) {
        let offset = self.chr_addr(addr); self.mem.chr.write(offset, value);
    }
}

state_fields!(Mmc3, mem,select,banks,ram_control,mirroring,latch,counter,reload,enabled,pending,a12,low_since);
