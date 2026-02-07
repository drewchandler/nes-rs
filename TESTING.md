Testing
=======

Unit tests run with:

```
cargo test
```

Integration ROM tests
---------------------

The integration tests in `tests/rom_harness.rs` are optional. They only run
when the ROM path environment variable is set or when `NES_TEST_ROM_DIR` points
to a directory containing the ROMs listed in `tests/roms.toml`. If the ROM path
is set but the expected hash variable is missing, the test fails and prints the
computed hash.

Environment variables:

- `NES_TEST_ROM_MEGAMAN1`: path to Mega Man 1 ROM
- `NES_EXPECTED_HASH_MEGAMAN1`: expected frame hash (hex, e.g. `0xdeadbeef`)
- `NES_TEST_FRAMES_MEGAMAN1`: optional frame count (default: 120)

For the blargg tests, `tests/roms.toml` defines ROM paths, SHA256 checks, and
frame counts. You can override these with:

- `NES_TEST_ROM_DIR`: directory containing the ROMs in `tests/roms.toml`
- `NES_EXPECTED_HASH_<ROM_ID>`: expected hash override
- `NES_TEST_FRAMES_<ROM_ID>`: frame count override
- `NES_TEST_ROM_<ROM_ID>`: ROM path override

Example ROM IDs: `BLARGG_PPU_VBL_CLEAR_TIME`, `BLARGG_PPU_VRAM_ACCESS`,
`BLARGG_PPU_PALETTE`, `BLARGG_PPU_SPRITE_HIT`.

Example (PowerShell):

```
$env:NES_TEST_ROM_MEGAMAN1="C:\roms\Mega Man.nes"
cargo test --test rom_harness -- megaman1_golden_frame
```
