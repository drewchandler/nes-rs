#[derive(Clone, Copy, Default)]
pub struct ButtonState {
    pub a: bool, pub b: bool, pub select: bool, pub start: bool,
    pub up: bool, pub down: bool, pub left: bool, pub right: bool,
}
impl ButtonState {
    pub fn from_bits(bits: u8) -> Self {
        ButtonState { a: bits&1!=0, b: bits&2!=0, select: bits&4!=0, start: bits&8!=0,
            up: bits&16!=0, down: bits&32!=0, left: bits&64!=0, right: bits&128!=0 }
    }
    pub fn bits(self) -> u8 { self.a as u8 | (self.b as u8)<<1 | (self.select as u8)<<2 | (self.start as u8)<<3 |
        (self.up as u8)<<4 | (self.down as u8)<<5 | (self.left as u8)<<6 | (self.right as u8)<<7 }
}
// $4016 high holds A on the data line. Its falling edge latches all eight
// buttons; subsequent reads shift that snapshot, then return ones indefinitely.
#[derive(Default)]
pub struct Joypad { state: ButtonState, shift: u8, high: bool }
impl Joypad {
    pub fn new() -> Self { Self::default() }
    pub fn strobe(&mut self, value: u8) {
        let high = value & 1 != 0;
        if high || self.high { self.shift = self.state.bits(); }
        self.high = high;
    }
    pub fn read(&mut self) -> u8 {
        if self.high { return self.state.a as u8; }
        let value = self.shift & 1; self.shift = (self.shift >> 1) | 0x80; value
    }
    pub fn set_state(&mut self, state: ButtonState) { self.state = state; }
}
state_fields!(ButtonState, a,b,select,start,up,down,left,right);
state_fields!(Joypad, state,shift,high);

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn strobe_latches_buttons_and_exhausted_reads_return_one() {
        let mut pad = Joypad::new(); pad.set_state(ButtonState::from_bits(0xa5));
        pad.strobe(1); for _ in 0..12 { assert_eq!(pad.read(),1); }
        pad.strobe(0); pad.set_state(ButtonState::from_bits(0));
        for bit in 0..8 { assert_eq!(pad.read(),(0xa5>>bit)&1); }
        for _ in 0..16 { assert_eq!(pad.read(),1); }
        pad.strobe(0); assert_eq!(pad.read(),1,"a low write does not relatch");
        pad.strobe(1); assert_eq!(pad.read(),0);
        pad.set_state(ButtonState::from_bits(1)); assert_eq!(pad.read(),1,"high reads live A");
    }
}
