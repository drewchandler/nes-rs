mod chr;
pub mod nrom;
pub mod unrom;

use std::cell::RefCell;
use std::rc::Rc;

// CPU register writes and PPU pattern fetches observe the same cartridge.
pub type Cartridge = Rc<RefCell<Box<dyn Mapper>>>;

pub trait Mapper {
    fn read(&self, addr: u16) -> u8;
    fn write(&mut self, addr: u16, value: u8);
    fn read_chr(&self, addr: u16) -> u8;
    fn write_chr(&mut self, addr: u16, value: u8);
}
