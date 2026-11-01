export const BUTTONS = ['A', 'B', 'Select', 'Start', 'Up', 'Down', 'Left', 'Right'];
export const DEFAULT_KEYS = [
  ['KeyZ', 'KeyX', 'ShiftRight', 'Enter', 'ArrowUp', 'ArrowDown', 'ArrowLeft', 'ArrowRight'],
  ['KeyG', 'KeyF', 'KeyR', 'KeyT', 'KeyW', 'KeyS', 'KeyA', 'KeyD'],
];
export const DEFAULT_PAD = [0, 1, 8, 9, 12, 13, 14, 15];
export class Controls {
  constructor(preferences) {
    const valid = preferences && Array.isArray(preferences.keys) && preferences.keys.length === 2 &&
      preferences.keys.every(row => Array.isArray(row) && row.length === 8 &&
        row.every(key => typeof key === 'string' && /^[A-Za-z0-9]{1,40}$/.test(key) && !['F5', 'F8'].includes(key))) &&
      Array.isArray(preferences.pads) && preferences.pads.length === 2 &&
      preferences.pads.every(row => Array.isArray(row) && row.length === 8 &&
        row.every(button => Number.isInteger(button) && button >= 0 && button < 64));
    this.keys = (valid ? preferences.keys : DEFAULT_KEYS).map(row => [...row]);
    this.pads = (valid ? preferences.pads : [DEFAULT_PAD, DEFAULT_PAD]).map(row => [...row]);
    this.held = new Set();
  }
  preferences() { return { keys: this.keys, pads: this.pads }; }
  clear() { this.held.clear(); }
  handles(code) { return this.keys.some(row => row.includes(code)); }
  bindKey(player, action, code) {
    const duplicate = this.keys[player].indexOf(code);
    if (duplicate !== -1) this.keys[player][duplicate] = this.keys[player][action];
    this.keys[player][action] = code;
  }
  read(gamepads = []) {
    const pads = [...gamepads].filter(pad => pad?.connected);
    return this.keys.map((keys, player) => {
      let bits = keys.reduce((mask, key, i) => mask | (this.held.has(key) ? 1 << i : 0), 0);
      const pad = pads[player];
      if (pad) {
        this.pads[player].forEach((button, i) => { if (pad.buttons[button]?.pressed) bits |= 1 << i; });
        if ((pad.axes[0] || 0) < -0.5) bits |= 64;
        if ((pad.axes[0] || 0) > 0.5) bits |= 128;
        if ((pad.axes[1] || 0) < -0.5) bits |= 16;
        if ((pad.axes[1] || 0) > 0.5) bits |= 32;
      }
      return bits;
    });
  }
}
