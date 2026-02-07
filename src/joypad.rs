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

enum StrobeState {
    A,
    B,
    Select,
    Start,
    Up,
    Down,
    Left,
    Right,
}

pub struct Joypad {
    state: ButtonState,
    strobe: StrobeState,
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
            strobe: StrobeState::A,
        }
    }

    pub fn strobe(&mut self) {
        self.strobe = StrobeState::A;
    }

    pub fn read(&mut self) -> u8 {
        let value = match self.strobe {
            StrobeState::A => {
                self.strobe = StrobeState::B;
                self.state.a
            }
            StrobeState::B => {
                self.strobe = StrobeState::Select;
                self.state.b
            }
            StrobeState::Select => {
                self.strobe = StrobeState::Start;
                self.state.select
            }
            StrobeState::Start => {
                self.strobe = StrobeState::Up;
                self.state.start
            }
            StrobeState::Up => {
                self.strobe = StrobeState::Down;
                self.state.up
            }
            StrobeState::Down => {
                self.strobe = StrobeState::Left;
                self.state.down
            }
            StrobeState::Left => {
                self.strobe = StrobeState::Right;
                self.state.left
            }
            StrobeState::Right => {
                self.strobe = StrobeState::A;
                self.state.right
            }
        };

        value as _
    }

    pub fn set_state(&mut self, state: ButtonState) {
        self.state = state;
    }
}
