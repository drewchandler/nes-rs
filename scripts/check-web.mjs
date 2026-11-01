import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';

const { instance: { exports: wasm } } = await WebAssembly.instantiate(
  await readFile(new URL('../dist/nes_rs.wasm', import.meta.url)), {}
);
function upload(bytes) {
  const pointer = wasm.rom_buffer(bytes.length);
  new Uint8Array(wasm.memory.buffer, pointer, bytes.length).set(bytes);
  return wasm.load_rom();
}
assert.equal(upload(new Uint8Array([1, 2, 3])), 1, 'Reject a malformed ROM');
const rom = new Uint8Array(16 + 32768);
rom.set([0x4e, 0x45, 0x53, 0x1a, 2]);
rom[6] = 0x50;
assert.equal(upload(rom), 2, 'Reject unsupported mapper 5');
rom[6] = 0x20;
rom.set([0x4c, 0x00, 0x80], 16); // JMP $8000
rom.set([0x00, 0x80], 16 + 32764); // Reset vector
assert.equal(upload(rom), 0, 'Load a synthetic mapper 2 cartridge');
for (let frame = 0; frame < 3; frame++) {
  const pointer = wasm.frame(frame === 0 ? 0 : 255);
  const pixels = new Uint8Array(wasm.memory.buffer, pointer, 256 * 240 * 4);
  assert.equal(pixels.length, 256 * 240 * 4);
  assert.equal(pixels[3], 255);
  if (frame > 0) assert.ok(wasm.audio_len() > 0, 'Produce audio samples');
  new Int16Array(wasm.memory.buffer, wasm.audio_ptr(), wasm.audio_len());
}
assert.equal(upload(rom), 0, 'Replace a running cartridge');
assert.ok(wasm.frame(0) > 0);
// Same NES 2.0 metadata profile as the reported ROM; all program bytes are synthetic.
rom[7] = 0x08;
rom[8] = 0x20;
rom[11] = 7;
assert.equal(upload(rom), 0, 'Load NES 2.0 UxROM with bus conflicts and 8 KiB CHR RAM');
assert.ok(wasm.frame(0) > 0);
rom[8] = 0x21;
assert.equal(upload(rom), 2, 'Extended mapper 258 must not alias mapper 2');
rom[8] = 0x20;
rom[9] = 1;
assert.equal(upload(rom), 2, 'Reject extended NES 2.0 sizes the parser does not implement');
rom[9] = 0;
rom[11] = 8;
assert.equal(upload(rom), 2, 'Reject unsupported CHR RAM layouts');
function nrom(prgBanks, chrBanks, nes2, bankWrites = []) {
  const prgBytes = prgBanks * 16384;
  const bytes = new Uint8Array(16 + prgBytes + chrBanks * 8192);
  bytes.set([0x4e, 0x45, 0x53, 0x1a, prgBanks, chrBanks, 1, nes2 ? 8 : 0]);
  if (nes2 && !chrBanks) bytes[11] = 7;
  if (prgBanks) {
    // Set a white background palette, attempt to erase the CHR pattern, then
    // enable rendering. CHR-ROM must stay intact for the frame to be white.
    const program = [
      0x78, 0xd8, ...bankWrites,
      0xa9, 0x3f, 0x8d, 6, 0x20,
      0xa9, 0, 0x8d, 6, 0x20,
      0xa9, 0x0f, 0x8d, 7, 0x20,
      0xa9, 0x30, 0x8d, 7, 0x20,
      0xa9, 0, 0x8d, 6, 0x20, 0x8d, 6, 0x20,
    ];
    for (let i = 0; i < 8; i++) program.push(0x8d, 7, 0x20);
    program.push(0x8d, 6, 0x20, 0x8d, 6, 0x20,
      0x8d, 5, 0x20, 0x8d, 5, 0x20, 0xa9, 0x0a, 0x8d, 1, 0x20);
    const loop = 0x8000 + program.length;
    program.push(0x4c, loop & 255, loop >> 8);
    bytes.set(program, 16);
    bytes.set([0, 0x80], 16 + prgBytes - 4);
    if (chrBanks) bytes.fill(255, 16 + prgBytes, 16 + prgBytes + 8);
  }
  return bytes;
}
for (const prgBanks of [1, 2]) {
  for (const nes2 of [false, true]) {
    const bytes = nrom(prgBanks, 1, nes2);
    assert.equal(upload(bytes), 0, 'Load NROM-128/256 with CHR-ROM in either header format');
    let pointer;
    for (let frame = 0; frame < 3; frame++) pointer = wasm.frame(0);
    const pixels = new Uint8Array(wasm.memory.buffer, pointer, 256 * 240 * 4);
    const offset = (100 * 256 + 100) * 4;
    assert.deepEqual([...pixels.slice(offset, offset + 4)], [255, 255, 255, 255],
      'CPU-driven rendering reads CHR-ROM and ignores pattern writes');
  }
}
assert.equal(upload(nrom(1, 0, true)), 0, 'Load NES 2.0 NROM with CHR RAM');
assert.equal(upload(nrom(3, 1, false)), 2, 'Reject NROM with unsupported PRG size');
assert.equal(upload(nrom(1, 2, false)), 2, 'Reject NROM with unsupported CHR size');
// Each supported controller boots actual CPU code in the compiled Wasm core.
// NES 2.0 metadata explicitly describes CHR RAM and mapper variants.
const profiles = [
  // mapper, PRG KiB, CHR-ROM KiB, CHR-RAM KiB, PRG-RAM KiB, submapper
  [0, 32, 8, 0, 0, 0], [1, 128, 128, 0, 8, 0], [2, 128, 0, 8, 0, 2],
  [3, 32, 32, 0, 0, 1], [4, 128, 128, 0, 8, 0], [7, 256, 0, 8, 0, 1],
  [9, 128, 128, 0, 8, 0], [10, 256, 128, 0, 8, 0], [11, 128, 128, 0, 0, 0],
  [13, 32, 0, 16, 0, 0], [32, 128, 128, 0, 8, 0], [34, 128, 0, 8, 0, 2],
  [38, 128, 32, 0, 0, 0], [66, 128, 32, 0, 0, 0], [70, 128, 128, 0, 0, 0],
  [71, 256, 0, 8, 0, 0], [75, 128, 128, 0, 0, 0], [78, 128, 128, 0, 0, 3],
  [79, 64, 64, 0, 0, 0], [87, 32, 32, 0, 0, 0], [89, 128, 128, 0, 0, 0],
  [93, 128, 0, 8, 0, 0], [94, 128, 0, 8, 0, 0], [140, 128, 128, 0, 0, 0],
  [152, 128, 128, 0, 0, 0], [180, 128, 0, 8, 0, 0], [206, 128, 64, 0, 0, 0],
  [232, 256, 0, 8, 0, 0],
  [34, 64, 64, 0, 8, 1], // NINA-001 and MMC6 have distinct memory wiring.
  [4, 128, 128, 0, 1, 1],
];
for (const [id, prg, chr, chrRam, prgRam, sub] of profiles) {
  const bytes = new Uint8Array(16 + (prg + chr) * 1024);
  bytes.set([0x4e, 0x45, 0x53, 0x1a, prg / 16, chr / 8,
    (id & 15) << 4, (id & 0xf0) | 8, sub << 4]);
  const ramShift = kb => kb ? Math.log2(kb * 1024) - 6 : 0;
  bytes[10] = ramShift(prgRam);
  bytes[11] = ramShift(chrRam);
  for (let bank = 0; bank < prg * 1024; bank += 8192) {
    bytes.set([0x4c, 0, 0x80], 16 + bank); // JMP $8000 in each power-on bank.
    bytes.set([0, 0x80], 16 + bank + 8192 - 4);
  }
  assert.equal(upload(bytes), 0, `Load mapper ${id}, submapper ${sub}`);
  for (let frame = 0; frame < 2; frame++) assert.ok(wasm.frame(0) > 0);
}
// Select a different CHR-ROM bank through a CPU register write. Only bank 1
// contains the white pattern; ignoring CNROM banking leaves a black frame.
const cnrom = nrom(2, 2, true, [0xa9, 1, 0x8d, 0xff, 0x8f]);
cnrom[6] = 0x31;
cnrom[8] = 0x10; // No bus conflicts.
cnrom.fill(0, 16 + 32768, 16 + 32768 + 8192);
cnrom.fill(255, 16 + 32768 + 8192, 16 + 32768 + 8192 + 8);
assert.equal(upload(cnrom), 0);
let cnromFrame;
for (let i = 0; i < 3; i++) cnromFrame = wasm.frame(0);
const cnromPixels = new Uint8Array(wasm.memory.buffer, cnromFrame, 256 * 240 * 4);
const center = (100 * 256 + 100) * 4;
assert.deepEqual([...cnromPixels.slice(center, center + 4)], [255, 255, 255, 255],
  'CNROM CPU bank register changes the PPU pattern memory');
console.log('Wasm video/audio, 28 mapper IDs, banked CHR rendering, and cartridge validation passed.');
