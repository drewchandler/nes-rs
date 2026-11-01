# Saves, states, and controls

## Game saves

The core preserves the ROM header's distinction between temporary RAM and
battery-backed RAM. Legacy iNES uses its battery flag; NES 2.0 uses the separate
PRG/CHR NVRAM size fields. Only nonvolatile bytes are written to game-save files.
A game still needs its own in-game save feature to update that memory.

Desktop loads a raw `.sav` next to the ROM and writes changes every two seconds
and on normal exit. For example, `Zelda.nes` uses `Zelda.sav`. Writes go through
a temporary file and replace the previous save after a complete write. An
unreadable or incorrectly sized existing save is kept, with automatic saving
paused for that run. Rename or repair it before restarting to enable saving.

The browser uses IndexedDB, keyed by the game's SHA-256 identity, independently
of the file's name. It saves changes every two seconds and attempts to flush
when hiding/leaving the page or choosing another game. A browser reload or ROM
replacement restores that game's stored save. Clearing the site's storage also
removes its saved games. Export `.sav` files from **Save files** for backups or
to exchange progress with the desktop app; import applies a raw save to the
currently loaded game and validates its size.

Denied, full, or blocked browser storage does not prevent playing. The page
reports the failure and keeps export available. A failed automatic restore
pauses automatic writes to protect the existing data. A valid manual import
can re-enable them. ROM files are never stored or uploaded by this feature.

## Save states

**Save state (F5)** captures the current CPU, PPU, APU channels/counters, RAM,
controller shift registers, mapper banks/IRQs, and regional clock phase.
**Load state (F8)** restores it. This also works for games without built-in
saving. Browser quick states persist per game in IndexedDB; if storage is
unavailable, the current tab keeps its most recent quick state in memory.
Desktop quick states use a `.nss` file beside the ROM.

Browser **Export state** / **Import state** transfers the same `.nss` format
used by desktop. The format is specific to this emulator. State files include
a format version, ROM/region identity, and checksum. Invalid, truncated,
corrupt, incompatible, and wrong-game states leave the running game intact.
The importer reconstructs a fresh machine and swaps it in only after validation.
The immutable ROM is excluded; the user must load the matching ROM first.

Restoring a state clears queued host audio and restarts its output resampler;
emulated APU and expansion-channel state is restored. A state restore can also
rewind the game's battery RAM, which will be saved by the next automatic write.

The versioned payload uses fixed-width little-endian integers, including
indices, so desktop and Wasm states share a layout. Changes to the serialized
hardware fields require a format-version change and either a migration or a
clear rejection of older states.

## Controls

Both controller ports support independent keyboard and gamepad input. The
standard controller strobe holds live A while high, latches all buttons on its
falling edge, shifts eight bits, and returns ones afterward. Host input changes
do not alter a latched read halfway through polling.

| Action | Player 1 | Player 2 |
| --- | --- | --- |
| A / B | Z / X | G / F |
| Select / Start | Right Shift / Enter | R / T |
| Move | Arrow keys | WASD |

The first two connected gamepads map to players 1 and 2; standard face buttons,
Select/Back, Start, D-pad, and left analog stick are supported. Disconnecting a
controller releases its input. Browser **Controls** remaps keys and buttons and
stores those preferences locally. Opening the dialog pauses emulation.

Desktop reads SDL's game-controller mappings. To remap keys or named gamepad
buttons, copy [native-controls.example](../native-controls.example), edit it,
and pass `--controls path/to/controls`. For example:

```sh
nes-rs game.nes --controls native-controls.example
nes-rs game.nes --region pal
```

Desktop Escape pauses; F5/F8 save/load states. Browser pause is on the page.
Choosing a new ROM restarts emulation. Its **TV system for the next game** menu
can override automatic NTSC/PAL/Dendy selection; desktop has `--region`.
