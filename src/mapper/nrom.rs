use mapper::{Mapper, memory::Memory};
use rom::Rom;
pub struct Nrom { mem: Memory }
impl Nrom { pub fn new(rom: Rom) -> Nrom { Nrom {mem:Memory::new(rom)} } }
impl Mapper for Nrom {
    fn memory(&self) -> Option<&Memory> { Some(&self.mem) }
    fn memory_mut(&mut self) -> Option<&mut Memory> { Some(&mut self.mem) }
    fn read(&self, addr: u16) -> u8 { assert!(addr>=0x8000); self.mem.prg(addr,32768,0) }
    fn write(&mut self, _addr: u16, _value: u8) {}
    fn read_low(&self, addr:u16) -> Option<u8> { self.mem.read_ram(addr) }
    fn write_low(&mut self, addr:u16, value:u8) { self.mem.write_ram(addr,value); }
    fn read_chr(&self, addr:u16) -> Option<u8> { Some(self.mem.chr.read(addr as usize)) }
    fn write_chr(&mut self, addr:u16, value:u8) { self.mem.chr.write(addr as usize,value); }
}
state_fields!(Nrom, mem);
#[cfg(test)] mod tests;
