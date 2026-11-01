let wasm;
async function init() {
  const response = await fetch(new URL('./nes_rs.wasm', import.meta.url));
  if (!response.ok) throw new Error('Wasm download failed: ' + response.status);
  ({ instance: { exports: wasm } } = await WebAssembly.instantiate(await response.arrayBuffer(), {}));
}
class BrowserEmulator {
  constructor(bytes) {
    const pointer = wasm.rom_buffer(bytes.length);
    new Uint8Array(wasm.memory.buffer, pointer, bytes.length).set(bytes);
    const result = wasm.load_rom();
    if (result) throw new Error(result === 1 ? 'Invalid or truncated iNES ROM.' :
      'Unsupported mapper, board variant, or memory layout. See the supported cartridge list below.');
  }
  frame(buttons) {
    const pointer = wasm.frame(buttons);
    return new Uint8Array(wasm.memory.buffer, pointer, 256 * 240 * 4).slice();
  }
  audio() {
    return new Int16Array(wasm.memory.buffer, wasm.audio_ptr(), wasm.audio_len()).slice();
  }
}

const picker = document.querySelector('#rom');
const pause = document.querySelector('#pause');
const mute = document.querySelector('#mute');
const status = document.querySelector('#status');
const canvas = document.querySelector('#screen');
const context = canvas.getContext('2d');
const keys = new Map([['KeyZ', 1], ['KeyX', 2], ['ShiftRight', 4], ['Enter', 8],
  ['ArrowUp', 16], ['ArrowDown', 32], ['ArrowLeft', 64], ['ArrowRight', 128]]);
const frameMs = 1000 / 60.0988;
let emulator, audioContext, gain, nextAudio = 0, buttons = 0;
let paused = false, muted = false, previous = 0, elapsed = 0, loadId = 0;
const sources = new Set();

function clearAudio() {
  for (const source of sources) source.stop();
  sources.clear();
  nextAudio = 0;
}
function updatePause() {
  buttons = 0;
  previous = 0;
  elapsed = 0;
  clearAudio();
  pause.textContent = paused ? 'Resume' : 'Pause';
}
function playAudio(samples) {
  if (!audioContext || audioContext.state !== 'running' || muted || !samples.length) return;
  const now = audioContext.currentTime;
  // Bound latency after stalls, backgrounding, or a slow device.
  if (nextAudio > now + 0.15) clearAudio();
  const buffer = audioContext.createBuffer(1, samples.length, 48000);
  const channel = buffer.getChannelData(0);
  for (let i = 0; i < samples.length; i++) channel[i] = samples[i] / 32768;
  const source = audioContext.createBufferSource();
  source.buffer = buffer;
  source.connect(gain);
  sources.add(source);
  source.onended = () => sources.delete(source);
  nextAudio = Math.max(nextAudio, now + 0.02);
  source.start(nextAudio);
  nextAudio += buffer.duration;
}
picker.addEventListener('change', async () => {
  const file = picker.files[0];
  if (!file) return;
  const id = ++loadId;
  picker.value = '';
  try {
    if (file.size > 8 * 1024 * 1024) throw new Error('ROM must be smaller than 8 MiB.');
    // Resume inside the user gesture for browser autoplay policies.
    try {
      if (!audioContext) {
        audioContext = new AudioContext({ sampleRate: 48000 });
        gain = audioContext.createGain();
        gain.connect(audioContext.destination);
      }
      await audioContext.resume();
    } catch { /* Video remains usable when audio is unavailable. */ }
    const bytes = new Uint8Array(await file.arrayBuffer());
    if (id !== loadId) return;
    const replacement = new BrowserEmulator(bytes);
    emulator = replacement;
    picker.blur();
    paused = false;
    updatePause();
    pause.disabled = false;
    mute.disabled = !audioContext;
    status.textContent = 'Playing ' + file.name +
      (audioContext?.state === 'running' ? '' : ' (sound unavailable)');
  } catch (error) {
    status.textContent = String(error);
  }
});
pause.addEventListener('click', async () => {
  paused = !paused;
  updatePause();
  if (!paused) await audioContext?.resume();
});
mute.addEventListener('click', () => {
  muted = !muted;
  clearAudio();
  mute.textContent = muted ? 'Unmute' : 'Mute';
});
window.addEventListener('keydown', event => {
  if (!emulator || paused || !keys.has(event.code) ||
      ['INPUT', 'BUTTON'].includes(event.target.tagName)) return;
  event.preventDefault();
  buttons |= keys.get(event.code);
});
window.addEventListener('keyup', event => { buttons &= ~(keys.get(event.code) || 0); });
window.addEventListener('blur', () => { buttons = 0; });
document.addEventListener('visibilitychange', updatePause);

function tick(now) {
  requestAnimationFrame(tick);
  if (!emulator || paused || document.hidden) { previous = 0; return; }
  elapsed += previous ? Math.min(now - previous, frameMs * 3) : frameMs;
  previous = now;
  try {
    while (elapsed >= frameMs) {
      const pixels = emulator.frame(buttons);
      context.putImageData(new ImageData(new Uint8ClampedArray(pixels), 256, 240), 0, 0);
      playAudio(emulator.audio());
      elapsed -= frameMs;
    }
  } catch (error) {
    paused = true;
    updatePause();
    emulator = undefined;
    picker.disabled = true;
    pause.disabled = true;
    status.textContent = 'Emulation stopped: ' + error + '. Choose another ROM to restart.';
    init().then(() => { picker.disabled = false; }).catch(error => { status.textContent = String(error); });
  }
}
try {
  await init();
  picker.disabled = false;
  status.textContent = 'Ready. Choose a ROM to play.';
  requestAnimationFrame(tick);
} catch (error) {
  status.textContent = 'Could not load emulator: ' + error;
}
