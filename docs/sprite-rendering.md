# Sprite rendering

The NTSC PPU supports 8x8 and 8x16 sprites, pattern-bank selection, four
sprite palettes, horizontal/vertical flips, OAM-order priority, background
priority, independent left-edge clipping, and the eight-sprite scanline limit.
Sprite colors share the background renderer's grayscale and RGB emphasis path.

Sprite Y is the top position minus one. Y=$FF does not wrap to scanline zero,
and sprites with Y >= 239 cannot produce a visible pixel. Pixels extending
past X=255 are clipped rather than wrapped.

## Pipeline

`src/ppu/sprites.rs` maintains secondary OAM and eight fetched sprite rows.
Evaluation runs alongside the current scanline's output and prepares the next
scanline:

| Dots | Work |
| --- | --- |
| 1-64 | Clear secondary OAM, one byte on each even dot. |
| 65-256 | Read primary OAM on odd dots; compare/copy on even dots. |
| 257-320 | Fetch eight slots, including dummy nametable reads and both pattern planes; force OAMADDR to zero. |
| Next visible scanline, 1-256 | Select the first opaque sprite pixel and compose it with the background. |

Transparency is determined by the two pattern bits, independently of palette
RGB. Sprite selection happens before background priority: an earlier sprite
hidden behind an opaque background still prevents a later sprite from showing.

Sprite-zero overlap becomes visible in PPUSTATUS on the following dot. It
requires opaque pixels from both enabled, unclipped layers, even when sprite
zero is behind the background. X=255 is excluded. Overflow evaluation includes
the hardware's diagonal scan through subsequent Y/tile/attribute/X bytes after
eight sprites are found. Both flags persist through status reads and vblank,
and clear at pre-render dot 1 even when rendering is disabled.

CPU OAMDATA reads mask the unimplemented attribute bits 2-4. Existing `$4014`
DMA feeds the same OAM used for sprite evaluation.

## Regression tests

Run the self-contained tests with:

```sh
cargo test --locked
cargo test --release --locked
```

The suite includes exhaustive low/high bitplane decoding, both sizes and banks,
all flips and palettes, scanline and screen-edge boundaries, priority and
transparency, flag timing/lifetime, overflow false positives/negatives, and a
real CPU-DMA-to-framebuffer test. Independent background/sprite reference
composition compares 983,040 pixels over 16 frames, including crowded scanlines,
clipping, offscreen sprites, and both sprite sizes. Existing background-only
reference comparisons remain enabled.

## Public conformance ROMs

ROM files are external fixtures, not bundled dependencies. These tests use
Shay Green's hardware-tested ROMs from `christopherpow/nes-test-roms`, revision
`95d8f621ae55cee0d09b91519a8989ae0e64753b`:

```sh
git clone https://github.com/christopherpow/nes-test-roms.git /tmp/nes-test-roms
git -C /tmp/nes-test-roms checkout 95d8f621ae55cee0d09b91519a8989ae0e64753b
NES_TEST_ROMS=/tmp/nes-test-roms cargo test --release --locked ppu::sprites::rom_tests -- --ignored --test-threads=1 --nocapture
```

The 17 external tests are ignored by the default suite. The command above runs
them explicitly in order: all 11 sprite-hit tests, the OAM RAM test, and all 5
sprite-overflow tests. Their source/readmes document each reported failure
code. The runner waits for the test shell's terminal self-jump, checks its
actual result byte ($F8 for hit/overflow; $F0 for the older OAM test), and fails
on a timeout. Intermediate result bytes are not treated as completed results.

The selected ROMs use the application's NROM mapper for fixed PRG and CHR-ROM
mapping. The real CPU, DMA, PPU registers, and shared CPU-cycle clock execute
the tests. The APU runs alongside the PPU; these tests do not open an audio device.

## Limits

The application shares [cartridge mapping](mappers.md) between CPU and PPU,
including banked CHR memory, dynamic mirroring, and MMC3 A12 IRQs. CPU bus
accesses advance the APU once and the PPU three dots on NTSC/Dendy or 16 dots
per five CPU cycles on PAL, including reset, interrupt, and DMA cycles. Passing
these sprite timing ROMs is not a claim of universal CPU/PPU race or half-cycle
bus accuracy. See [audio and shared timing](audio.md) for the clock integration.

Rendering-time OAMDATA bus values, OAM corruption, nonzero-OAMADDR evaluation
quirks, and OAM writes during rendering are not modeled fully. Evaluation
stops once overflow is latched because further reads cannot change the selected
sprites or the flag; those remaining internal bus reads are not exposed.
Regional frame/scanline timing is supported. PAL OAM refresh/corruption details
and analog composite color behavior remain outside this renderer's scope.
