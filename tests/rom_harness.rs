extern crate nes_rs;
extern crate sha2;
extern crate toml;

use nes_rs::joypad::ButtonState;
use nes_rs::nes::Nes;
use nes_rs::rom::Rom;
use sha2::{Digest, Sha256};
use std::env;
use std::fs::File;
use std::io::Read;
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

#[derive(Clone, Debug)]
struct RomSpec {
    id: String,
    path: String,
    sha256: String,
    frames: usize,
    expected_hash: Option<String>,
}

fn load_rom_specs() -> Vec<RomSpec> {
    let data = include_str!("roms.toml");
    let value: toml::Table = toml::from_str(data).expect("Failed to parse tests/roms.toml");
    let roms = value
        .get("roms")
        .and_then(|roms| roms.as_array())
        .expect("roms array missing from tests/roms.toml");

    roms.iter()
        .map(|rom| {
            let id = rom
                .get("id")
                .and_then(|value| value.as_str())
                .expect("rom id missing")
                .to_string();
            let path = rom
                .get("path")
                .and_then(|value| value.as_str())
                .expect("rom path missing")
                .to_string();
            let sha256 = rom
                .get("sha256")
                .and_then(|value| value.as_str())
                .expect("rom sha256 missing")
                .to_lowercase();
            let frames = rom
                .get("frames")
                .and_then(|value| value.as_integer())
                .unwrap_or(300) as usize;
            let expected_hash = rom
                .get("expected_hash")
                .and_then(|value| value.as_str())
                .map(|value| value.to_string());

            RomSpec {
                id,
                path,
                sha256,
                frames,
                expected_hash,
            }
        })
        .collect()
}

fn rom_spec_by_id(id: &str) -> RomSpec {
    let specs = load_rom_specs();
    specs
        .into_iter()
        .find(|spec| spec.id == id)
        .unwrap_or_else(|| panic!("Missing ROM spec for {}", id))
}

fn env_key(prefix: &str, id: &str) -> String {
    format!("{}_{}", prefix, id.to_uppercase())
}

fn sha256_file(path: &Path) -> String {
    let mut file = File::open(path).expect("Failed to open ROM file for hashing");
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 8192];
    loop {
        let read = file.read(&mut buf).expect("Failed to read ROM file");
        if read == 0 {
            break;
        }
        hasher.update(&buf[..read]);
    }
    format!("{:x}", hasher.finalize())
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
            panic!("Missing {}. Computed hash: {:016x}", expected_env, hash);
        }
    };

    assert_eq!(
        hash, expected,
        "Frame hash mismatch for {} after {} frames (expected {:016x}, got {:016x})",
        rom_env, frames, expected, hash
    );
}

fn assert_manifest_frame(id: &str) {
    let spec = rom_spec_by_id(id);

    let rom_env = env_key("NES_TEST_ROM", &spec.id);
    let expected_env = env_key("NES_EXPECTED_HASH", &spec.id);
    let frames_env = env_key("NES_TEST_FRAMES", &spec.id);

    let path = if let Some(path) = rom_path_from_env(&rom_env) {
        path
    } else if let Ok(dir) = env::var("NES_TEST_ROM_DIR") {
        let candidate = PathBuf::from(dir).join(&spec.path);
        if !candidate.exists() {
            panic!("ROM missing at {} for {}", candidate.display(), spec.id);
        }
        candidate
    } else {
        eprintln!("Skipping {}: set {} or NES_TEST_ROM_DIR.", spec.id, rom_env);
        return;
    };

    let actual_sha = sha256_file(&path);
    assert_eq!(
        actual_sha, spec.sha256,
        "SHA mismatch for {} (expected {}, got {})",
        spec.id, spec.sha256, actual_sha
    );

    let frames = env::var(frames_env)
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(spec.frames);

    let hash = run_rom_for_frames(&path, frames);

    let expected = match env::var(&expected_env) {
        Ok(value) => parse_hex_u64(&value).expect("Expected hash must be hex"),
        Err(_) => {
            let value = spec.expected_hash.as_ref().unwrap_or_else(|| {
                panic!("Missing {}. Computed hash: {:016x}", expected_env, hash)
            });
            parse_hex_u64(value).expect("Expected hash must be hex")
        }
    };

    assert_eq!(
        hash, expected,
        "Frame hash mismatch for {} after {} frames (expected {:016x}, got {:016x})",
        spec.id, frames, expected, hash
    );
}

#[test]
fn blargg_ppu_vbl_clear_time_golden_frame() {
    assert_manifest_frame("blargg_ppu_vbl_clear_time");
}

#[test]
fn blargg_ppu_vram_access_golden_frame() {
    assert_manifest_frame("blargg_ppu_vram_access");
}

#[test]
fn blargg_ppu_palette_golden_frame() {
    assert_manifest_frame("blargg_ppu_palette");
}

#[test]
fn blargg_ppu_sprite_hit_golden_frame() {
    assert_manifest_frame("blargg_ppu_sprite_hit");
}
