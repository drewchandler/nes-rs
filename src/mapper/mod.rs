mod chr;
mod memory;
mod factory;
mod discrete;
mod mmc1;
mod mmc2;
mod mmc3;
mod registers;
pub use self::factory::{create, supported, SUPPORTED};
pub mod nrom;
pub mod unrom;

use std::cell::RefCell;
use std::rc::Rc;
use rom::Mirroring;

// CPU register writes and PPU pattern fetches observe the same cartridge.
pub type Cartridge = Rc<RefCell<Box<dyn Mapper>>>;

pub trait Mapper {
    fn read(&self, addr: u16) -> u8;
    fn write(&mut self, addr: u16, value: u8);
    fn read_low(&self, _addr: u16) -> Option<u8> { None }
    fn write_low(&mut self, _addr: u16, _value: u8) {}
    fn write_cpu(&mut self, addr: u16, value: u8, _cycle: u64) { self.write(addr, value); }
    fn mirroring(&self) -> Option<Mirroring> { None }
    fn ppu_address(&mut self, _addr: u16, _cycle: u64) {}
    fn irq(&self) -> bool { false }
    fn read_chr(&self, addr: u16) -> Option<u8>;
    fn write_chr(&mut self, addr: u16, value: u8);
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod rom_tests;
