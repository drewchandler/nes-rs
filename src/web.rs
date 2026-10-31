// A small numeric ABI keeps the static frontend independent of binding tools.
// JavaScript copies ROM bytes into rom_buffer, then calls load_rom. Returned
// pointers remain valid until the next call that mutates the corresponding Vec.
use std::cell::RefCell;
use std::io::Cursor;
use joypad::ButtonState;
use nes::Nes;
use rom::Rom;

#[derive(Default)]
struct Browser {
    rom: Vec<u8>,
    nes: Option<Nes>,
    pixels: Vec<u8>,
    samples: Vec<i16>,
}
thread_local! { static BROWSER: RefCell<Browser> = RefCell::new(Browser::default()); }

#[no_mangle]
pub extern "C" fn rom_buffer(length: usize) -> *mut u8 {
    BROWSER.with(|state| {
        let mut state = state.borrow_mut();
        state.rom.resize(length, 0);
        state.rom.as_mut_ptr()
    })
}

// 1 = malformed/truncated ROM; 2 = unsupported cartridge; 0 = ready.
#[no_mangle]
pub extern "C" fn load_rom() -> u32 {
    BROWSER.with(|state| {
        let mut state = state.borrow_mut();
        let rom = match Rom::read(&mut Cursor::new(&state.rom)) {
            Ok(rom) => rom,
            Err(_) => return 1,
        };
        if state.rom[7] & 0x0c != 0 || rom.mapper != 2 ||
            rom.prg_rom.len() < 2 || !rom.chr_rom.is_empty() { return 2; }
        let mut nes = Nes::new(rom);
        nes.reset();
        state.nes = Some(nes);
        0
    })
}

// Bits: A, B, Select, Start, Up, Down, Left, Right.
#[no_mangle]
pub extern "C" fn frame(buttons: u8) -> *const u8 {
    BROWSER.with(|state| {
        let mut state = state.borrow_mut();
        let state = &mut *state;
        let nes = state.nes.as_mut().expect("Load a ROM before running");
        let screen = nes.run_frame(ButtonState {
            a: buttons & 1 != 0, b: buttons & 2 != 0,
            select: buttons & 4 != 0, start: buttons & 8 != 0,
            up: buttons & 16 != 0, down: buttons & 32 != 0,
            left: buttons & 64 != 0, right: buttons & 128 != 0,
        });
        state.pixels.clear();
        for &pixel in screen.iter() {
            state.pixels.extend_from_slice(&[(pixel >> 16) as u8, (pixel >> 8) as u8, pixel as u8, 255]);
        }
        state.samples = nes.interconnect.apu.take_samples();
        state.pixels.as_ptr()
    })
}

#[no_mangle]
pub extern "C" fn audio_ptr() -> *const i16 {
    BROWSER.with(|state| state.borrow().samples.as_ptr())
}

#[no_mangle]
pub extern "C" fn audio_len() -> usize {
    BROWSER.with(|state| state.borrow().samples.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn upload(bytes: &[u8]) {
        rom_buffer(bytes.len());
        BROWSER.with(|state| state.borrow_mut().rom.copy_from_slice(bytes));
    }

    #[test]
    fn rejects_bad_and_unsupported_roms_then_runs_a_valid_cartridge() {
        std::thread::Builder::new().stack_size(8 * 1024 * 1024)
            .spawn(check_cartridge_lifecycle).unwrap().join().unwrap();
    }

    fn check_cartridge_lifecycle() {
        upload(b"bad");
        assert_eq!(load_rom(), 1);
        let mut bytes = vec![0; 16 + 32768];
        bytes[..4].copy_from_slice(b"NES\x1a");
        bytes[4] = 2;
        upload(&bytes);
        assert_eq!(load_rom(), 2);
        bytes[6] = 0x20;
        // JMP $8000, with reset vector pointing at it.
        bytes[16..19].copy_from_slice(&[0x4c, 0, 0x80]);
        bytes[16 + 32764..16 + 32766].copy_from_slice(&[0, 0x80]);
        upload(&bytes);
        assert_eq!(load_rom(), 0);
        assert!(!frame(0).is_null());
        BROWSER.with(|state| {
            let state = state.borrow();
            assert_eq!(state.pixels.len(), 256 * 240 * 4);
            assert!(state.pixels.chunks(4).all(|pixel| pixel[3] == 255));
        });
        assert!(!audio_ptr().is_null());
        // The first PPU frame can finish before the APU's first output block.
        frame(0xff);
        assert!(audio_len() > 0);
        // Replacing a running cartridge resets cleanly.
        assert_eq!(load_rom(), 0);
        assert!(!frame(0).is_null());
    }
}
