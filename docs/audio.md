# NES sound

The emulator implements the NTSC 2A03 APU and plays 48 kHz mono audio through
SDL2. Pulse, triangle, noise, and DMC output feed the NES nonlinear mixer,
band-limited resampling, and approximate 90/440 Hz high-pass and 14 kHz low-pass
filters. No audio device is required to emulate the APU or run its unit tests.

## Build and run

In addition to minifb's existing platform requirements, building bundled SDL2
requires a C/C++ toolchain and CMake on `PATH`. SDL2 is statically linked; a
separate SDL development installation is not required. Tested on Linux with
Rust 1.85.1 and CMake 3.31.6.

```sh
cargo run --release --locked -- /path/to/game.nes
```

Playback uses the default audio device. About two frames of queued audio pace
emulation, with a 1024-sample device buffer. A missing or stalled device produces
one message and falls back to timed video playback. SDL owns only audio;
minifb handles the window and input, and normal process signals retain their
default behavior.

In this managed environment, the existing `/workspace/.tools/env.sh` injects
an X11 header for minifb 0.9. SDL's feature macros must precede that header:

```sh
. /workspace/.tools/env.sh
export PATH=/workspace/.tools/cmake/cmake/data/bin:$PATH
export CFLAGS="-D_GNU_SOURCE=1 $CFLAGS"
cargo build --release --locked
```

The extra compiler flag is specific to that environment's preincluded header.

## Channels and registers

| Unit | Behavior |
| --- | --- |
| Pulse 1/2 | Four duty cycles, envelopes/constant volume, length counters, sweep dividers and muting, channel-specific negate arithmetic |
| Triangle | 32-step waveform, linear and length counters, held DAC when gated |
| Noise | 15-bit LFSR, long/short feedback taps, 16 NTSC periods, envelope and length |
| DMC | 16 NTSC rates, saturating 7-bit DAC, LSB-first decoding, retained sample buffer, mapped CPU reads, address wrap, looping, IRQs |
| Frame sequencer | Four/five-step modes, quarter/half clocks, delayed `$4017` reset, three frame-IRQ assertions, length write/clock collisions |
| `$4015` | Channel enables and active-counter status; reading clears frame IRQ, writing clears DMC IRQ; bit 5 retains CPU open bus |
| `$4017` | APU frame-counter writes; controller-port reads |

Disabling channels clears length counters, not their timers. Triangle and DMC
DACs retain their levels. DMC status tracks unread sample bytes, so a cleared
status bit does not mean its buffered tail has finished sounding.

## Shared timing

`Interconnect::cpu_read` and `cpu_write` clock the APU once and PPU three dots.
CPU instruction fetches, dummy accesses, stack accesses, reset, interrupts, and
DMA all use these methods. Raw `read_word`/`write_word` accesses support setup
and inspection without advancing time. `Nes::run_frame` waits for the PPU's
frame boundary; callers must not advance the PPU again after `Cpu::step`.

APU frame/DMC IRQs feed CPU interrupt polling. CLI, SEI, and PLP use the prior
I flag; RTI uses the restored flag. NMI takes priority over a sampled IRQ.
DMC halts CPU reads, waits through CPU writes, and takes three/four cycles
depending on alignment. During sprite DMA, a DMC get replaces an OAM get and
OAM retries after the idle put. OAM alignment accounts for DMC steals during
its startup. Elapsed cycles include all stalls.

The mixer retains fractional sample timing across frame boundaries. Core audio
buffering is capped at 100 ms so headless callers can discard sound without
unbounded memory use. Normal playback drains samples every video frame.

## Validation

The default suite includes channel waveforms and divider behavior, sweep
arithmetic, envelope progression, all length entries and enable gates, linear
counter gating, both noise sequence periods, DMC decoding/saturation/buffering,
IRQ clearing, and independent nonlinear mixer calculations. Resampling tests
check chunk-boundary invariance, sample count, DC rejection, and ultrasonic
alias rejection. Synthetic ROMs running through the real CPU/frame loop check
pulse, triangle, and DMC pitches. DMA tests sweep request timing across both
CPU parities and confirm all sprite bytes survive simultaneous DMC transfers.

```sh
cargo test --locked
```

Optional fixtures come from
[christopherpow/nes-test-roms](https://github.com/christopherpow/nes-test-roms)
at revision `95d8f621ae55cee0d09b91519a8989ae0e64753b`:

```sh
git clone https://github.com/christopherpow/nes-test-roms.git /tmp/nes-test-roms
git -C /tmp/nes-test-roms checkout 95d8f621ae55cee0d09b91519a8989ae0e64753b
NES_TEST_ROMS=/tmp/nes-test-roms cargo test --release --locked \
  apu::rom_tests -- --ignored --test-threads=1 --nocapture
```

All eight `apu_test` singles, eleven `blargg_apu_2005.07.30` tests, and six
`apu_reset` tests pass. The adapter reproduces fixed NROM PRG/CHR and supplies
the test shell's SRAM, while executing the application's CPU, APU, DMA, and PPU
clock paths. Modern shells require the `$6001..$6003` signature and final
`$6000` result; reset requests receive a delayed soft reset. Older shells must
reach their terminal self-jump and report success at `$F0`. The separate old
`dmc_tests` files are listening tests without this result protocol and are not
counted as automated passes.

The existing seventeen sprite/OAM ROMs also pass through the shared clock.
To run every test, including a headless SDL queue/pacing check:

```sh
SDL_AUDIODRIVER=dummy NES_TEST_ROMS=/tmp/nes-test-roms \
  cargo test --release --locked -- --include-ignored --test-threads=1
```

Linux window smoke checks also verified sprite/background output, graceful
window close and process termination, audio-device failure, and PCM captured
through SDL's disk driver. A pulse programmed for 440.40 Hz measured 440.26 Hz
in the captured stream. This verifies the playback path without a physical
speaker in the cloud environment.

## Limits

The application targets NTSC, mapper 2, and CHR RAM. PAL/Dendy timing, expansion
audio, additional mappers, and mapper IRQs require further work. The analog
output/filter model is an approximation.

CPU-cycle clocking does not model every half-cycle race. Rare DMC abort/start
and DMA arbitration cases are not fully modeled; controller-read glitches and
IRQ/NMI vector-hijacking races are outside this validation. Passing these ROMs
does not establish full hardware bus accuracy. Rendering-time OAM bus and
corruption limits are described in [sprite rendering](sprite-rendering.md).
