import { Emulator } from './core.mjs';
import { Controls, BUTTONS } from './controls.mjs';
import { SaveStorage, equalBytes } from './storage.mjs';

const $ = selector => document.querySelector(selector);
const picker = $('#rom'), demo = $('#demo'), pause = $('#pause'), mute = $('#mute');
const status = $('#status'), notice = $('#notice'), canvas = $('#screen');
const context = canvas.getContext('2d'), dialog = $('#controls-dialog');
const storage = new SaveStorage();
let preferences;
try { preferences = JSON.parse(localStorage.getItem('nes-rs-controls')); } catch { /* Defaults work without storage. */ }
let controls = new Controls(preferences);
let wasm, game, audioContext, gain, nextAudio = 0, lastCheck = 0;
let paused = false, muted = false, previous = 0, elapsed = 0, loadId = 0, binding;
let saveQueue = Promise.resolve();
const sources = new Set();
const demos = { '2048': ['2048.nes', '2048'], 'concentration-room': ['concentration-room.nes', 'Concentration Room'] };

function message(text) { notice.textContent = text; }
function clearAudio() {
  for (const source of sources) source.stop();
  sources.clear(); nextAudio = 0;
}
function resetClock() { controls.clear(); previous = 0; elapsed = 0; clearAudio(); }
function focusGame() { canvas.focus({ preventScroll: true }); }
function updatePause() { resetClock(); pause.textContent = paused ? 'Resume' : 'Pause'; }
function updateActions() {
  for (const element of document.querySelectorAll('[data-needs-game]')) element.disabled = !game;
  $('#export-save').disabled = !game?.hasBattery;
  $('#import-save').disabled = !game?.hasBattery;
}
async function resumeAudio() {
  try {
    if (!audioContext) { audioContext = new AudioContext({ sampleRate: 48000 }); gain = audioContext.createGain(); gain.connect(audioContext.destination); }
    await audioContext.resume();
  } catch { /* Video stays usable when audio is unavailable. */ }
}
function playAudio(samples) {
  if (!audioContext || audioContext.state !== 'running' || muted || !samples.length) return;
  if (nextAudio > audioContext.currentTime + 0.15) clearAudio();
  const buffer = audioContext.createBuffer(1, samples.length, 48000), channel = buffer.getChannelData(0);
  for (let i = 0; i < samples.length; i++) channel[i] = samples[i] / 32768;
  const source = audioContext.createBufferSource(); source.buffer = buffer; source.connect(gain);
  sources.add(source); source.onended = () => sources.delete(source);
  nextAudio = Math.max(nextAudio, audioContext.currentTime + 0.02); source.start(nextAudio); nextAudio += buffer.duration;
}
function persistBattery(current = game) {
  if (!current || current !== game || !current.hasBattery || !current.autosave) return saveQueue;
  const bytes = current.core.export('battery');
  if (equalBytes(bytes, current.pendingBattery)) return saveQueue;
  current.pendingBattery = bytes;
  // Snapshot the bytes and identity before any asynchronous work or ROM replacement.
  saveQueue = saveQueue.catch(() => {}).then(() => storage.put('battery', current.id, bytes)).catch(() => {
    current.pendingBattery = null; current.autosave = false;
    if (game === current) message('Automatic saves are unavailable. Use Export game save to keep your progress.');
  });
  return saveQueue;
}
async function loadGame(bytes, name, ticket) {
  if (ticket !== loadId) return;
  if (bytes.length > 8 * 1024 * 1024) throw new Error('ROM must be smaller than 8 MiB.');
  const wasPaused = paused;
  paused = true; updatePause();
  await persistBattery();
  if (ticket !== loadId) return;
  let core;
  try { core = new Emulator(wasm, bytes, Number($('#region').value)); }
  catch (error) { paused = wasPaused; updatePause(); throw error; }
  const current = { core, name, id: core.id, hasBattery: core.export('battery').length > 0, autosave: true };
  game = current; message('');
  if (current.hasBattery) {
    try {
      const saved = await storage.get('battery', current.id);
      if (ticket !== loadId || game !== current) return;
      if (saved) { core.import('battery', saved); current.pendingBattery = saved; }
    } catch { current.autosave = false; if (ticket === loadId && game === current) message('Could not restore an automatic save. Automatic saving is paused to protect it. Import a game save or export your progress.'); }
  }
  if (ticket !== loadId || game !== current) return;
  paused = false; updatePause(); updateActions(); focusGame();
  status.textContent = `Playing ${name} · ${core.fps.toFixed(2)} fps`;
}
async function chooseFile() {
  const file = picker.files[0]; picker.value = ''; if (!file) return;
  const ticket = ++loadId; status.textContent = `Loading ${file.name}…`;
  try {
    if (file.size > 8 * 1024 * 1024) throw new Error('ROM must be smaller than 8 MiB.');
    await resumeAudio(); await loadGame(new Uint8Array(await file.arrayBuffer()), file.name, ticket);
  }
  catch (error) { if (ticket === loadId) { message(error.message); status.textContent = game ? `Playing ${game.name}` : 'Ready. Choose another ROM.'; } }
}
picker.addEventListener('change', chooseFile);
demo.addEventListener('change', async () => {
  const selected = demos[demo.value]; demo.value = ''; if (!selected) return;
  const ticket = ++loadId; status.textContent = `Loading ${selected[1]}…`;
  try {
    await resumeAudio();
    const response = await fetch(new URL(`./roms/${selected[0]}`, import.meta.url));
    if (!response.ok) throw new Error('Could not load the demo game.');
    await loadGame(new Uint8Array(await response.arrayBuffer()), selected[1], ticket);
  } catch (error) { if (ticket === loadId) { message(error.message); status.textContent = game ? `Playing ${game.name}` : 'Ready. Choose another ROM.'; } }
});
pause.addEventListener('click', async () => { paused = !paused; updatePause(); if (!paused) await resumeAudio(); focusGame(); });
mute.addEventListener('click', () => { muted = !muted; clearAudio(); mute.textContent = muted ? 'Unmute' : 'Mute'; focusGame(); });

function download(bytes, extension) {
  const url = URL.createObjectURL(new Blob([bytes], { type: 'application/octet-stream' }));
  const link = document.createElement('a'); link.href = url;
  link.download = game.name.replace(/\.[^.]+$/, '') + extension; link.click();
  setTimeout(() => URL.revokeObjectURL(url), 1000); focusGame();
}
$('#export-save').addEventListener('click', () => download(game.core.export('battery'), '.sav'));
$('#export-state').addEventListener('click', () => download(game.core.export('state'), '.nss'));
for (const [id, kind] of [['import-save', 'battery'], ['import-state', 'state']]) {
  $("#" + id).addEventListener('change', async event => {
    const file = event.target.files[0], current = game; event.target.value = ''; if (!file || !current) return;
    try {
      if (file.size > 4 * 1024 * 1024) throw new Error('Save file is too large.');
      const bytes = new Uint8Array(await file.arrayBuffer()); if (game !== current) return;
      current.core.import(kind, bytes); current.autosave = true; current.pendingBattery = null; resetClock();
      if (kind === 'state') current.memoryState = bytes;
      await persistBattery(); message((kind === 'state' ? 'State restored.' : 'Game save imported.') + (current.hasBattery && !current.autosave ? ' Automatic saving is unavailable; export your progress.' : ''));
    } catch (error) { message(error.message); }
    focusGame();
  });
}
async function saveState() {
  const current = game; if (!current) return;
  const bytes = current.core.export('state'); current.memoryState = bytes;
  try { await storage.put('state', current.id, bytes); if (game === current) message('State saved.'); }
  catch { if (game === current) message('State saved for this tab only. Export the state to keep it.'); }
  focusGame();
}
async function loadState() {
  const current = game; if (!current) return;
  try {
    let bytes = current.memoryState;
    if (!bytes) bytes = await storage.get('state', current.id);
    if (game !== current) return;
    if (!bytes) throw new Error('No saved state for this game yet.');
    current.core.import('state', bytes); current.autosave = true; current.pendingBattery = null; resetClock(); await persistBattery(); message('State restored.');
  } catch (error) { message(error.message); }
  focusGame();
}
$('#save-state').addEventListener('click', saveState);
$('#load-state').addEventListener('click', loadState);

function saveControls() {
  try { localStorage.setItem('nes-rs-controls', JSON.stringify(controls.preferences())); }
  catch { message('Controls apply to this tab; browser preferences could not be saved.'); }
}
function renderControls() {
  const body = $('#bindings'); body.replaceChildren();
  for (let player = 0; player < 2; player++) for (let action = 0; action < 8; action++) {
    const row = document.createElement('tr');
    for (const value of [`${player + 1}`, BUTTONS[action]]) { const cell = document.createElement('td'); cell.textContent = value; row.append(cell); }
    for (const kind of ['key', 'pad']) {
      const cell = document.createElement('td'), button = document.createElement('button'); button.type = 'button';
      button.textContent = kind === 'key' ? controls.keys[player][action] : `Button ${controls.pads[player][action]}`;
      button.addEventListener('click', () => {
        binding = { player, action, kind, previous: [...(navigator.getGamepads?.() || [])].filter(p => p?.connected)[player]?.buttons.map(b => b.pressed) || [] };
        $('#binding-status').textContent = `Player ${player + 1} ${BUTTONS[action]}: press a ${kind === 'key' ? 'key' : 'controller button'}. Escape cancels.`;
      }); cell.append(button); row.append(cell);
    }
    body.append(row);
  }
}
$('#controls').addEventListener('click', () => { resetClock(); renderControls(); dialog.showModal(); });
$('#reset-controls').addEventListener('click', () => { controls = new Controls(); binding = null; saveControls(); renderControls(); });
dialog.addEventListener('close', () => { binding = null; resetClock(); focusGame(); });
window.addEventListener('keydown', event => {
  if (binding) {
    event.preventDefault();
    if (event.code === 'Escape') { binding = null; $('#binding-status').textContent = 'Remapping cancelled.'; }
    else if (event.code === 'F5' || event.code === 'F8') { $('#binding-status').textContent = 'F5 and F8 are reserved for save states. Choose another key.'; }
    else if (binding.kind === 'key' && /^[A-Za-z0-9]{1,40}$/.test(event.code)) {
      controls.bindKey(binding.player, binding.action, event.code); binding = null;
      saveControls(); renderControls(); $('#binding-status').textContent = 'Key saved.';
    }
    return;
  }
  if (dialog.open || !game || ['INPUT', 'BUTTON', 'SELECT'].includes(event.target.tagName)) return;
  if (event.code === 'F5' || event.code === 'F8') { event.preventDefault(); if (!event.repeat) (event.code === 'F5' ? saveState : loadState)(); return; }
  if (paused || !controls.handles(event.code)) return;
  event.preventDefault(); controls.held.add(event.code);
});
window.addEventListener('keyup', event => controls.held.delete(event.code));
window.addEventListener('blur', resetClock);
document.addEventListener('visibilitychange', () => { persistBattery(); resetClock(); });
window.addEventListener('pagehide', () => { persistBattery(); });

function tick(now) {
  requestAnimationFrame(tick);
  const pads = navigator.getGamepads?.() || [];
  if (dialog.open && binding?.kind === 'pad') {
    const pad = [...pads].filter(p => p?.connected)[binding.player];
    const pressed = pad?.buttons.findIndex((button, i) => button.pressed && !binding.previous[i]) ?? -1;
    if (pressed !== -1) {
      controls.pads[binding.player][binding.action] = pressed; binding = null;
      saveControls(); renderControls(); $('#binding-status').textContent = 'Controller button saved.';
    } else if (pad) binding.previous = pad.buttons.map(button => button.pressed);
  }
  if (!game || paused || document.hidden || dialog.open) { previous = 0; return; }
  const frameMs = 1000 / game.core.fps;
  elapsed += previous ? Math.min(now - previous, frameMs * 3) : frameMs; previous = now;
  try {
    while (elapsed >= frameMs) {
      const [first, second] = controls.read(pads), pixels = game.core.frame(first, second);
      context.putImageData(new ImageData(pixels, 256, 240), 0, 0); playAudio(game.core.audio()); elapsed -= frameMs;
    }
    if (now - lastCheck >= 2000) { lastCheck = now; persistBattery(); }
  } catch (error) {
    paused = true; updatePause(); message(`Emulation stopped: ${error.message}. Reload the page to restart.`);
  }
}
try {
  const response = await fetch(new URL('./nes_rs.wasm', import.meta.url));
  if (!response.ok) throw new Error(`Wasm download failed: ${response.status}`);
  ({ instance: { exports: wasm } } = await WebAssembly.instantiate(await response.arrayBuffer(), {}));
  picker.disabled = false; demo.disabled = false;
  status.textContent = 'Ready. Choose a demo or load your own ROM.'; updateActions(); requestAnimationFrame(tick);
} catch (error) { status.textContent = `Could not load emulator: ${error.message}`; }
