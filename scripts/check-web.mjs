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
assert.equal(upload(rom), 2, 'Reject unsupported mapper 0');
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
console.log('Wasm cartridge loading, video, audio, input, replacement, and NES 2.0 validation passed.');
