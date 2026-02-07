extern crate nes_rs;

use nes_rs::joypad::ButtonState;
use nes_rs::nes::Nes;
use nes_rs::rom::Rom;
use std::env;
use std::path::{Path, PathBuf};

fn rom_path_from_env(var: &str) -> Option<PathBuf> {
    let value = match env::var(var) {
        Ok(value) => value,
        Err(_) => return None,
    };
    let path = PathBuf::from(value);
    if !path.exists() {
        panic!("ROM path for {} does not exist: {}", var, path.display());
    }
    Some(path)
}

fn parse_hex_u64(value: &str) -> Result<u64, String> {
    let value = value.trim();
    let value = value.strip_prefix("0x").unwrap_or(value);
    u64::from_str_radix(value, 16).map_err(|err| format!("Invalid hex {}: {}", value, err))
}

fn button_state_idle() -> ButtonState {
    ButtonState {
        a: false,
        b: false,
        select: false,
        start: false,
        up: false,
        down: false,
        left: false,
        right: false,
    }
}

fn frame_hash(frame: &[u32]) -> u64 {
    const FNV_OFFSET: u64 = 0xcbf29ce484222325;
    const FNV_PRIME: u64 = 0x100000001b3;
    let mut hash = FNV_OFFSET;
    for pixel in frame {
        hash ^= *pixel as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

fn run_rom_for_frames(path: &Path, frames: usize) -> u64 {
    let rom = Rom::load(path).expect("Failed to load ROM");
    let mut nes = Nes::new(rom);
    nes.reset();

    let buttons = button_state_idle();
    let mut hash = 0u64;
    for _ in 0..frames {
        let frame = nes.run_frame(buttons);
        hash = frame_hash(frame);
    }
    hash
}

fn assert_golden_frame(rom_env: &str, expected_env: &str, frames_env: &str, default_frames: usize) {
    let path = match rom_path_from_env(rom_env) {
        Some(path) => path,
        None => {
            eprintln!(
                "Skipping {}: set {} (ROM path) and {} (expected hash).",
                rom_env, rom_env, expected_env
            );
            return;
        }
    };

    let frames = env::var(frames_env)
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(default_frames);

    let hash = run_rom_for_frames(&path, frames);

    let expected = match env::var(expected_env) {
        Ok(value) => parse_hex_u64(&value).expect("Expected hash must be hex"),
        Err(_) => {
            panic!(
                "Missing {}. Computed hash: {:016x}",
                expected_env, hash
            );
        }
    };

    assert_eq!(
        hash, expected,
        "Frame hash mismatch for {} after {} frames (expected {:016x}, got {:016x})",
        rom_env, frames, expected, hash
    );
}

#[test]
fn megaman1_golden_frame() {
    assert_golden_frame(
        "NES_TEST_ROM_MEGAMAN1",
        "NES_EXPECTED_HASH_MEGAMAN1",
        "NES_TEST_FRAMES_MEGAMAN1",
        120,
    );
}

#[test]
fn blargg_ppu_vbl_nmi_golden_frame() {
    assert_golden_frame(
        "NES_TEST_ROM_BLARGG_PPU_VBL_NMI",
        "NES_EXPECTED_HASH_BLARGG_PPU_VBL_NMI",
        "NES_TEST_FRAMES_BLARGG_PPU_VBL_NMI",
        300,
    );
}

#[test]
fn blargg_ppu_scroll_golden_frame() {
    assert_golden_frame(
        "NES_TEST_ROM_BLARGG_PPU_SCROLL",
        "NES_EXPECTED_HASH_BLARGG_PPU_SCROLL",
        "NES_TEST_FRAMES_BLARGG_PPU_SCROLL",
        300,
    );
}

#[test]
fn blargg_ppu_palette_golden_frame() {
    assert_golden_frame(
        "NES_TEST_ROM_BLARGG_PPU_PALETTE",
        "NES_EXPECTED_HASH_BLARGG_PPU_PALETTE",
        "NES_TEST_FRAMES_BLARGG_PPU_PALETTE",
        300,
    );
}

#[test]
fn blargg_ppu_sprite_hit_golden_frame() {
    assert_golden_frame(
        "NES_TEST_ROM_BLARGG_PPU_SPRITE_HIT",
        "NES_EXPECTED_HASH_BLARGG_PPU_SPRITE_HIT",
        "NES_TEST_FRAMES_BLARGG_PPU_SPRITE_HIT",
        300,
    );
}
