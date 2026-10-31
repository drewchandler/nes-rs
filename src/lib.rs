extern crate blip_buf;
#[cfg(not(target_arch = "wasm32"))]
extern crate minifb;
#[cfg(not(target_arch = "wasm32"))]
extern crate sdl2;

mod apu;
#[cfg(not(target_arch = "wasm32"))]
mod audio;
mod cpu;
#[cfg(not(target_arch = "wasm32"))]
pub mod emulator;
mod interconnect;
mod joypad;
mod mapper;
mod nes;
mod ppu;
pub mod rom;
#[cfg(any(test, target_arch = "wasm32"))]
mod web;
