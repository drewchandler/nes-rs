Testing
=======

Unit tests run with:

```
cargo test
```

Integration ROM tests
---------------------

The integration tests in `tests/rom_harness.rs` are optional. They only run
when the ROM path environment variable is set. If the ROM path is set but the
expected hash variable is missing, the test fails and prints the computed hash.

Environment variables:

- `NES_TEST_ROM_MEGAMAN1`: path to Mega Man 1 ROM
- `NES_EXPECTED_HASH_MEGAMAN1`: expected frame hash (hex, e.g. `0xdeadbeef`)
- `NES_TEST_FRAMES_MEGAMAN1`: optional frame count (default: 120)

- `NES_TEST_ROM_BLARGG_PPU_VBL_NMI`
- `NES_EXPECTED_HASH_BLARGG_PPU_VBL_NMI`
- `NES_TEST_FRAMES_BLARGG_PPU_VBL_NMI` (default: 300)

- `NES_TEST_ROM_BLARGG_PPU_SCROLL`
- `NES_EXPECTED_HASH_BLARGG_PPU_SCROLL`
- `NES_TEST_FRAMES_BLARGG_PPU_SCROLL` (default: 300)

- `NES_TEST_ROM_BLARGG_PPU_PALETTE`
- `NES_EXPECTED_HASH_BLARGG_PPU_PALETTE`
- `NES_TEST_FRAMES_BLARGG_PPU_PALETTE` (default: 300)

- `NES_TEST_ROM_BLARGG_PPU_SPRITE_HIT`
- `NES_EXPECTED_HASH_BLARGG_PPU_SPRITE_HIT`
- `NES_TEST_FRAMES_BLARGG_PPU_SPRITE_HIT` (default: 300)

Example (PowerShell):

```
$env:NES_TEST_ROM_MEGAMAN1="C:\roms\Mega Man.nes"
cargo test --test rom_harness -- megaman1_golden_frame
```
