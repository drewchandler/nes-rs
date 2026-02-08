#[derive(Clone, Copy)]
pub struct ButtonState {
    pub a: bool,
    pub b: bool,
    pub select: bool,
    pub start: bool,
    pub up: bool,
    pub down: bool,
    pub left: bool,
    pub right: bool,
}

pub struct Joypad {
    state: ButtonState,
    strobe: bool,
    shift: u8,
}

impl Joypad {
    pub fn new() -> Joypad {
        Joypad {
            state: ButtonState {
                a: false,
                b: false,
                select: false,
                start: false,
                up: false,
                down: false,
                left: false,
                right: false,
            },
            strobe: false,
            shift: 0,
        }
    }

    pub fn write_strobe(&mut self, value: u8) {
        let new_strobe = value & 1 != 0;
        if new_strobe || self.strobe {
            self.latch();
        }
        self.strobe = new_strobe;
    }

    pub fn read(&mut self) -> u8 {
        if self.strobe {
            return self.state.a as u8;
        }

        let value = self.shift & 1;
        self.shift = (self.shift >> 1) | 0x80;
        value
    }

    pub fn set_state(&mut self, state: ButtonState) {
        self.state = state;
    }

    fn latch(&mut self) {
        self.shift = self.state_to_bits();
    }

    fn state_to_bits(&self) -> u8 {
        (self.state.a as u8)
            | ((self.state.b as u8) << 1)
            | ((self.state.select as u8) << 2)
            | ((self.state.start as u8) << 3)
            | ((self.state.up as u8) << 4)
            | ((self.state.down as u8) << 5)
            | ((self.state.left as u8) << 6)
            | ((self.state.right as u8) << 7)
    }
}

impl Default for Joypad {
    fn default() -> Self {
        Joypad::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strobe_high_reads_a() {
        let mut joypad = Joypad::new();
        joypad.set_state(ButtonState {
            a: true,
            b: true,
            select: false,
            start: false,
            up: false,
            down: false,
            left: false,
            right: false,
        });

        joypad.write_strobe(1);
        assert_eq!(joypad.read(), 1);
        assert_eq!(joypad.read(), 1);
    }

    #[test]
    fn test_shift_sequence_after_strobe() {
        let mut joypad = Joypad::new();
        joypad.set_state(ButtonState {
            a: true,
            b: false,
            select: true,
            start: false,
            up: true,
            down: false,
            left: true,
            right: false,
        });

        joypad.write_strobe(1);
        joypad.write_strobe(0);

        let expected = [1, 0, 1, 0, 1, 0, 1, 0];
        for &bit in &expected {
            assert_eq!(joypad.read(), bit);
        }
    }
}
