mod chr;
mod memory;
mod factory;
mod discrete;
mod mmc1;
mod mmc2;
mod mmc3;
mod registers;
mod expansion;
mod vrc6;
mod fme7;
pub use self::factory::{create, supported, SUPPORTED};
pub mod nrom;
pub mod unrom;

use std::cell::RefCell;
use std::rc::Rc;
use rom::Mirroring;

// CPU register writes and PPU pattern fetches observe the same cartridge.
pub type Cartridge = Rc<RefCell<Box<dyn Mapper>>>;

pub trait Mapper: ::state::Snapshot {
    fn memory(&self) -> Option<&memory::Memory> { None }
    fn memory_mut(&mut self) -> Option<&mut memory::Memory> { None }
    fn battery(&self) -> Vec<u8> { self.memory().map_or(Vec::new(), |m| m.battery()) }
    fn load_battery(&mut self, bytes: &[u8]) -> ::state::Result<()> {
        if let Some(m) = self.memory_mut() {m.load_battery(bytes)} else if bytes.is_empty() {Ok(())} else {Err("This cartridge has no battery RAM".into())}
    }
    fn read(&self, addr: u16) -> u8;
    fn write(&mut self, addr: u16, value: u8);
    fn read_low(&self, _addr: u16) -> Option<u8> { None }
    fn write_low(&mut self, _addr: u16, _value: u8) {}
    fn write_cpu(&mut self, addr: u16, value: u8, _cycle: u64) { self.write(addr, value); }
    fn mirroring(&self) -> Option<Mirroring> { None }
    fn ppu_address(&mut self, _addr: u16, _cycle: u64) {}
    fn clock_cpu(&mut self) {}
    fn audio(&self) -> f32 {0.0}
    fn ciram_page(&self, _table:usize) -> Option<usize> {None}
    fn nametable_read(&self, _addr:u16) -> Option<u8> {None}
    fn nametable_write(&mut self, _addr:u16, _value:u8) -> bool {false}
    fn irq(&self) -> bool { false }
    fn read_chr(&self, addr: u16) -> Option<u8>;
    fn write_chr(&mut self, addr: u16, value: u8);
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod rom_tests;
