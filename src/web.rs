// A small numeric ABI keeps the static frontend independent of binding tools.
// JavaScript copies ROM bytes into rom_buffer, then calls load_rom. Returned
// pointers remain valid until the next call that mutates the corresponding Vec.
use joypad::ButtonState;
use nes::Nes;
use rom::Rom;
use std::cell::RefCell;
use std::io::Cursor;

#[derive(Default)]
struct Browser {
    rom: Vec<u8>,
    nes: Option<Nes>,
    pixels: Vec<u8>,
    samples: Vec<i16>,
    transfer: Vec<u8>,
    error: String,
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

// Mapper-specific RAM/ROM and submapper rules are shared with desktop builds.
// Console and timing variants still need separate CPU/PPU implementations.
fn supported_header(header: &[u8]) -> bool {
    match header[7] & 0x0c {
        0 => header[7] & 3 == 0,
        0x08 => {
            header[7] & 3 == 0
                && header[9] == 0
                && header[12] <= 3
                && header[13] == 0
                && header[14] == 0
        }
        _ => false,
    }
}

// 1 = malformed/truncated ROM; 2 = unsupported cartridge; 0 = ready.
#[no_mangle]
pub extern "C" fn load_rom() -> u32 {
    load_rom_region(0)
}
#[no_mangle]
pub extern "C" fn load_rom_region(region: u32) -> u32 {
    BROWSER.with(|state| {
        let mut state = state.borrow_mut();
        if state.rom.len() < 16 || state.rom[..4] != *b"NES\x1a" {
            return 1;
        }
        if !supported_header(&state.rom[..16]) {
            return 2;
        }
        let mut rom = match Rom::read(&mut Cursor::new(&state.rom)) {
            Ok(rom) => rom,
            Err(_) => return 1,
        };
        match region {
            0 => {}
            1 => rom.region = ::region::Region::Ntsc,
            2 => rom.region = ::region::Region::Pal,
            3 => rom.region = ::region::Region::Dendy,
            _ => return 2,
        }
        if !::mapper::supported(&rom) {
            return 2;
        }
        let mut nes = Nes::new(rom);
        nes.reset();
        state.nes = Some(nes);
        0
    })
}

// Bits: A, B, Select, Start, Up, Down, Left, Right.
#[no_mangle]
pub extern "C" fn frame(buttons: u8) -> *const u8 {
    frame_two(buttons, 0)
}
#[no_mangle]
pub extern "C" fn frame_two(buttons: u8, second: u8) -> *const u8 {
    BROWSER.with(|state| {
        let mut state = state.borrow_mut();
        let state = &mut *state;
        let nes = state.nes.as_mut().expect("Load a ROM before running");
        let screen = nes.run_frame_two(
            ButtonState::from_bits(buttons),
            ButtonState::from_bits(second),
        );
        state.pixels.clear();
        for &pixel in screen.iter() {
            state.pixels.extend_from_slice(&[
                (pixel >> 16) as u8,
                (pixel >> 8) as u8,
                pixel as u8,
                255,
            ]);
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

#[no_mangle]
pub extern "C" fn frame_rate() -> f64 {
    BROWSER.with(|s| s.borrow().nes.as_ref().map_or(60.0988, |n| n.fps()))
}
#[no_mangle]
pub extern "C" fn rom_id() -> *const u8 {
    BROWSER.with(|s| {
        s.borrow()
            .nes
            .as_ref()
            .map_or(::std::ptr::null(), |n| n.rom_id.as_ptr())
    })
}
#[no_mangle]
pub extern "C" fn transfer_buffer(length: usize) -> *mut u8 {
    if length > 4 * 1024 * 1024 {
        return ::std::ptr::null_mut();
    }
    BROWSER.with(|s| {
        let mut s = s.borrow_mut();
        s.transfer.resize(length, 0);
        s.transfer.as_mut_ptr()
    })
}
#[no_mangle]
pub extern "C" fn transfer_len() -> usize {
    BROWSER.with(|s| s.borrow().transfer.len())
}
#[no_mangle]
pub extern "C" fn battery_export() -> *const u8 {
    BROWSER.with(|s| {
        let mut s = s.borrow_mut();
        s.transfer = s.nes.as_ref().map_or(Vec::new(), |n| n.battery());
        s.transfer.as_ptr()
    })
}
#[no_mangle]
pub extern "C" fn state_export() -> *const u8 {
    BROWSER.with(|s| {
        let mut s = s.borrow_mut();
        s.transfer = s.nes.as_ref().map_or(Vec::new(), |n| n.save_state());
        s.transfer.as_ptr()
    })
}
fn import(state: bool) -> u32 {
    BROWSER.with(|s| {
        let mut s = s.borrow_mut();
        let s = &mut *s;
        let result = match s.nes.as_mut() {
            Some(n) => {
                if state {
                    n.load_state(&s.transfer)
                } else {
                    n.load_battery(&s.transfer)
                }
            }
            None => Err("Load a game first".into()),
        };
        match result {
            Ok(()) => {
                s.error.clear();
                s.samples.clear();
                0
            }
            Err(message) => {
                s.error = message;
                1
            }
        }
    })
}
#[no_mangle]
pub extern "C" fn battery_import() -> u32 {
    import(false)
}
#[no_mangle]
pub extern "C" fn state_import() -> u32 {
    import(true)
}
#[no_mangle]
pub extern "C" fn error_ptr() -> *const u8 {
    BROWSER.with(|s| s.borrow().error.as_ptr())
}
#[no_mangle]
pub extern "C" fn error_len() -> usize {
    BROWSER.with(|s| s.borrow().error.len())
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
        std::thread::Builder::new()
            .stack_size(8 * 1024 * 1024)
            .spawn(check_cartridge_lifecycle)
            .unwrap()
            .join()
            .unwrap();
    }

    fn check_cartridge_lifecycle() {
        upload(b"bad");
        assert_eq!(load_rom(), 1);
        let mut bytes = vec![0; 16 + 32768];
        bytes[..4].copy_from_slice(b"NES\x1a");
        bytes[4] = 2;
        bytes[6] = 0x50; // MMC5 remains unsupported.
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
    #[test]
    fn nes2_uxrom_header_loads_without_accepting_unsupported_layouts() {
        std::thread::Builder::new()
            .stack_size(8 * 1024 * 1024)
            .spawn(|| {
                // Metadata matching the reported cartridge; payload is synthetic.
                let mut bytes = vec![0; 16 + 8 * 16384];
                bytes[..16].copy_from_slice(&[
                    0x4e, 0x45, 0x53, 0x1a, 8, 0, 0x21, 0x08, 0x20, 0, 0, 7, 0, 0, 0, 1,
                ]);
                bytes[16..19].copy_from_slice(&[0x4c, 0, 0x80]);
                let reset_high = bytes.len() - 3;
                bytes[reset_high] = 0x80;
                upload(&bytes);
                assert_eq!(load_rom(), 0);
                assert!(!frame(0).is_null());
                // Unsupported extended mapper, submapper, sizes, RAM, and timing.
                for &(offset, value) in &[
                    (8, 0x21),
                    (8, 0x30),
                    (9, 1),
                    (10, 7),
                    (11, 8),
                    (12, 4),
                    (7, 0x0c),
                ] {
                    let original = bytes[offset];
                    bytes[offset] = value;
                    upload(&bytes);
                    assert_eq!(load_rom(), 2, "header byte {}", offset);
                    bytes[offset] = original;
                }
                upload(&bytes[..16]);
                assert_eq!(load_rom(), 1, "truncated supported NES 2.0 cartridge");
            })
            .unwrap()
            .join()
            .unwrap();
    }

    fn nrom_image(prg_banks: u8, chr_banks: u8, nes2: bool) -> Vec<u8> {
        let prg_bytes = prg_banks as usize * 16384;
        let mut bytes = vec![0; 16 + prg_bytes + chr_banks as usize * 8192];
        bytes[..4].copy_from_slice(b"NES\x1a");
        bytes[4] = prg_banks;
        bytes[5] = chr_banks;
        if nes2 {
            bytes[7] = 8;
            bytes[11] = if chr_banks == 0 { 7 } else { 0 };
        }
        if prg_bytes != 0 {
            bytes[16..19].copy_from_slice(&[0x4c, 0, 0x80]);
            bytes[16 + prg_bytes - 4..16 + prg_bytes - 2].copy_from_slice(&[0, 0x80]);
        }
        bytes
    }

    #[test]
    fn nrom_loads_supported_memory_layouts_and_rejects_invalid_bank_counts() {
        std::thread::Builder::new()
            .stack_size(8 * 1024 * 1024)
            .spawn(|| {
                for &nes2 in &[false, true] {
                    for prg_banks in 1..=2 {
                        for chr_banks in 0..=1 {
                            upload(&nrom_image(prg_banks, chr_banks, nes2));
                            assert_eq!(load_rom(), 0);
                            assert!(!frame(0).is_null());
                        }
                    }
                    for &(prg_banks, chr_banks) in &[(0, 1), (3, 1), (1, 2)] {
                        upload(&nrom_image(prg_banks, chr_banks, nes2));
                        assert_eq!(load_rom(), 2);
                    }
                }
                let mut bytes = nrom_image(1, 1, true);
                bytes[8] = 0x10; // NROM has no implemented submapper 1.
                upload(&bytes);
                assert_eq!(load_rom(), 2);
                bytes[8] = 0;
                bytes[11] = 7; // Simultaneous CHR-ROM and CHR RAM is unsupported.
                upload(&bytes);
                assert_eq!(load_rom(), 2);
            })
            .unwrap()
            .join()
            .unwrap();
    }
}
