// The same numeric ABI is exercised by Node smoke tests and the browser.
export class Emulator {
  constructor(wasm, bytes, region = 0) {
    this.wasm = wasm;
    const pointer = wasm.rom_buffer(bytes.length);
    new Uint8Array(wasm.memory.buffer, pointer, bytes.length).set(bytes);
    const result = wasm.load_rom_region(region);
    if (result) throw new Error(result === 1 ? 'Invalid or truncated iNES ROM.' :
      'Unsupported mapper, board variant, or memory layout. See Supported cartridges.');
    this.id = [...new Uint8Array(wasm.memory.buffer, wasm.rom_id(), 32)]
      .map(byte => byte.toString(16).padStart(2, '0')).join('');
    this.fps = wasm.frame_rate();
  }
  frame(first, second) {
    const w = this.wasm, pointer = w.frame_two(first, second);
    return new Uint8ClampedArray(w.memory.buffer, pointer, 256 * 240 * 4).slice();
  }
  audio() {
    const w = this.wasm;
    return new Int16Array(w.memory.buffer, w.audio_ptr(), w.audio_len()).slice();
  }
  export(kind) {
    const w = this.wasm, pointer = w[`${kind}_export`]();
    return new Uint8Array(w.memory.buffer, pointer, w.transfer_len()).slice();
  }
  import(kind, bytes) {
    const w = this.wasm, pointer = w.transfer_buffer(bytes.length);
    if (!pointer) throw new Error('File is too large.');
    new Uint8Array(w.memory.buffer, pointer, bytes.length).set(bytes);
    if (w[`${kind}_import`]()) {
      throw new Error(new TextDecoder().decode(new Uint8Array(w.memory.buffer, w.error_ptr(), w.error_len())));
    }
  }
}
