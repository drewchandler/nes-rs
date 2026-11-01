# Cartridge support

Desktop and WebAssembly builds use the same cartridge factory and CPU/PPU
memory. There are 28 supported mapper IDs. An ID identifies a family of boards;
the submapper and memory sizes must also match a profile below. Unsupported
profiles are rejected instead of guessing a different board's wiring.

## Boards

Sizes are KiB. ROM sizes must be powers of two, in 16 KiB PRG and 8 KiB CHR
units. CHR RAM sizes are explicit in NES 2.0; legacy iNES defaults to 8 KiB when
there is no CHR-ROM, or 16 KiB for CPROM. Optional PRG RAM is at most 8 KiB
unless noted. Boards without PRG RAM ignore the legacy header's implicit RAM
default but reject NES 2.0 headers that explicitly request it.

| ID | Board | PRG ROM | Graphics memory / variants |
| --- | --- | --- | --- |
| 0 | NROM | 16–32 | 8 CHR-ROM or RAM; submapper 0 |
| 1 | MMC1 / SxROM | 16–512 | Up to 128 CHR-ROM or 8 RAM; submapper 0; submapper 5 fixes 32 PRG |
| 2 | UxROM | 16–2048 | 8 CHR RAM; submappers 0/1/2 |
| 3 | CNROM | 16–32 | Up to 32 CHR-ROM; optional PRG RAM; submappers 0/1/2 |
| 4 | MMC3 / MMC6 | 32–512 | Up to 256 CHR-ROM or 8 RAM; variants below |
| 7 | AxROM | 32–256 | 8 CHR RAM, one-screen mirroring; submappers 0/1/2 |
| 9 | MMC2 / PxROM | 32–128 | Up to 128 CHR-ROM, pattern-read latches; optional PRG RAM |
| 10 | MMC4 / FxROM | 32–256 | Up to 128 CHR-ROM, pattern-read latches; optional PRG RAM |
| 11 | Color Dreams | 32–128 | Up to 128 CHR-ROM; submappers 0/1/2 |
| 13 | CPROM | 32 | 16 CHR RAM, fixed lower 4 / switchable upper 4, vertical mirroring |
| 32 | Irem G-101 | 32–256 | Up to 256 CHR-ROM or 8 RAM; optional PRG RAM; submapper 1 fixes one-screen upper |
| 34 | BNROM / NINA-001 | 32–128 BNROM; 32–64 NINA | BNROM: 8 CHR RAM; NINA-001: up to 64 CHR-ROM and optional PRG RAM |
| 38 | Bit Corp | 32–128 | Up to 32 CHR-ROM; registers at $7000–$7FFF |
| 66 | GxROM | 32–128 | Up to 32 CHR-ROM |
| 70 | Bandai 74161/7432 | 32–128 | Up to 128 CHR-ROM, header mirroring |
| 71 | Camerica | 32–256 | 8 CHR RAM; submapper 1 adds Fire Hawk one-screen control |
| 75 | Konami VRC1 | 32–128 | Up to 128 CHR-ROM, independent 4 KiB banks |
| 78 | Irem 74HC161/32 | 32–128 | Up to 128 CHR-ROM; submapper 1: Cosmo Carrier; 3: Holy Diver |
| 79 | NINA-03/06 | 32–64 | Up to 64 CHR-ROM; low CPU register decoding |
| 87 | Jaleco | 16–32 | Up to 32 CHR-ROM, swapped CHR bank bits |
| 89 | Sunsoft-2 | 32–128 | Up to 128 CHR-ROM, one-screen control |
| 93 | Sunsoft-2 | 32–128 | 8 CHR RAM with chip enable / floating reads when disabled |
| 94 | UN1ROM | 32–128 | 8 CHR RAM, PRG select in bits 2–4 |
| 140 | Jaleco JF-11/14 | 32–128 | Up to 128 CHR-ROM; registers at $6000–$7FFF |
| 152 | Bandai 74161/7432 | 32–128 | Up to 128 CHR-ROM, one-screen control |
| 180 | UNROM-180 | 16–128 | 8 CHR RAM, first PRG bank fixed / upper bank switchable |
| 206 | DxROM / Namco 108 | 32–128 | Up to 64 CHR-ROM; no MMC3 mode flips, RAM, or IRQs |
| 232 | Camerica Quattro | 64–256 | 8 CHR RAM, independent outer/inner PRG selection |

Unlisted submappers are zero. Mapper 34 submapper 1 selects NINA-001; 2
selects BNROM; unspecified legacy/submapper 0 uses CHR-ROM presence. Mapper
78's ambiguous submapper 0 is accepted only in legacy iNES: the historical
four-screen flag selects Holy Diver's horizontal/vertical control, otherwise
Cosmo Carrier's one-screen control is used. Legacy mapper 71 accepts the
historical Fire Hawk mirroring write at $9000–$9FFF.

UxROM, CNROM, AxROM, and Color Dreams honor NES 2.0's explicit conflict
selection: submapper 1 has no bus conflicts, 2 ANDs writes with the current PRG
ROM byte. Unspecified UxROM/AxROM/Color Dreams retain no-conflict compatibility;
unspecified CNROM uses AND conflicts. Other discrete boards with bus conflicts
apply them before decoding data bits. Each board retains its own register
address decoding and bank-bit wiring.

## MMC1 and MMC3 details

MMC1 supports serial register writes, consecutive-cycle D0 filtering, D7 reset,
16/32 KiB PRG modes, 4/8 KiB CHR modes, RAM disable, and dynamic one-screen /
horizontal / vertical mirroring. CHR-RAM boards include SNROM RAM disable,
SUROM's outer 256 KiB PRG selection, and SOROM/SXROM's 16/32 KiB PRG RAM
banking. A 512 KiB PRG profile requires CHR RAM. CHR-ROM profiles support up to
8 KiB PRG RAM; board wiring is inferred from the declared memory sizes.
Special NES-EVENT, 2ME, MMC1A-only RAM behavior, and mixed CHR-ROM/RAM boards
are outside these profiles. Software using outer banking in 4 KiB CHR mode
must keep the outer bits in both CHR registers consistent.

MMC3 supports inverted PRG/CHR windows, RAM enable/write protection,
horizontal/vertical mirroring, hardwired four-screen RAM, and IRQs driven by
filtered PPU A12 rises. CPU accesses through PPUADDR/PPUDATA and rendering
fetches both reach the cartridge. Internal palette lookup does not clock IRQs.

| Mapper 4 submapper | Behavior |
| --- | --- |
| 0 | Sharp MMC3 counter behavior, including reload-to-zero IRQs |
| 1 | MMC6 with 1 KiB PRG RAM at $7000–$7FFF, independent half-bank read/write protection |
| 2 | MMC3 with header-controlled hardwired mirroring |
| 4 | NEC counter behavior; automatic zero-to-zero reload suppresses IRQ, explicit reload can assert it |

MMC6 clears RAM protection while global RAM enable is off, and a half-bank
can be written only when both its read and write bits are enabled. With just
one half enabled for reading, the other drives zero; both disabled leave open
bus. MC-ACC (submapper 3), scramblers, TQROM mixed graphics memory, and other
MMC3-derived boards are rejected.

## Save RAM and format limits

PRG RAM and battery-backed PRG NVRAM are allocated for the current cartridge,
including MMC1 banks and MMC6's small RAM. Saves are not persisted to disk,
browser storage, or a file: restarting/replacing a cartridge clears them.
NES 2.0 volatile and nonvolatile RAM sizes share this session-only storage.

The parser accepts basic iNES / NES 2.0 with ordinary bank counts; NES 2.0
extended/exponent ROM sizes and mixed CHR-ROM/CHR RAM are not supported.
The browser additionally rejects non-NTSC-compatible timing and alternate
consoles. No expansion audio is implemented. Trainers are skipped rather than
installed into PRG RAM. Mapper support does not imply every game or every
CPU/PPU half-cycle race has been verified.

## Validation

The default native suite covers bank signatures for every controller, separate
low-address decoders, bus conflicts, dynamic mirroring through real CPU PPU
ports, serial-write timing, latch-triggering reads, PRG RAM protection/banking,
and PPU-driven IRQ delivery. The existing Wasm smoke test boots synthetic ROMs
for all 28 IDs plus NINA-001 and MMC6, and checks CPU-driven CNROM banked
rendering. No commercial ROMs are bundled or needed by CI.

Six optional MMC3 conformance ROMs from Shay Green pass, including exact
scanline timing for both background/sprite pattern-table configurations and
Sharp / NEC counter differences. These use the production CPU, PPU, APU, bus,
and mapper without an SRAM or timing adapter. They are ignored by the default
suite and do not add a CI job or release-time tests.

```sh
git clone https://github.com/christopherpow/nes-test-roms.git /tmp/nes-test-roms
git -C /tmp/nes-test-roms checkout 95d8f621ae55cee0d09b91519a8989ae0e64753b
NES_TEST_ROMS=/tmp/nes-test-roms cargo test --release --locked \
  mapper::rom_tests -- --ignored --test-threads=1 --nocapture
```

The runner requires the $6001–$6003 signature and a completed $6000 result,
fails on timeout, and selects each ROM's expected controller revision. This
checks the six `mmc3_test_2/rom_singles` files, not an entire library of games.
Board register and variant definitions follow [NESdev's mapper documentation](https://www.nesdev.org/wiki/Mapper),
[MMC1](https://www.nesdev.org/wiki/MMC1),
[MMC3](https://www.nesdev.org/wiki/MMC3),
[MMC6](https://www.nesdev.org/wiki/MMC6), and
[NES 2.0 submappers](https://www.nesdev.org/wiki/NES_2.0_submappers).
