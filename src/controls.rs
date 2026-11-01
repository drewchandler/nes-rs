use joypad::ButtonState;
use minifb::{Key, Window};
use sdl2::controller::{Axis, Button, GameController};
use sdl2::{self, GameControllerSubsystem, Sdl};
use std::path::Path;

const NAMES: [&str; 8] = ["A", "B", "Select", "Start", "Up", "Down", "Left", "Right"];
const PAD: [Button; 8] = [
    Button::A,
    Button::B,
    Button::Back,
    Button::Start,
    Button::DPadUp,
    Button::DPadDown,
    Button::DPadLeft,
    Button::DPadRight,
];
fn key(name: &str) -> Option<Key> {
    let letters = [
        Key::A,
        Key::B,
        Key::C,
        Key::D,
        Key::E,
        Key::F,
        Key::G,
        Key::H,
        Key::I,
        Key::J,
        Key::K,
        Key::L,
        Key::M,
        Key::N,
        Key::O,
        Key::P,
        Key::Q,
        Key::R,
        Key::S,
        Key::T,
        Key::U,
        Key::V,
        Key::W,
        Key::X,
        Key::Y,
        Key::Z,
    ];
    if name.len() == 1 {
        let b = name.as_bytes()[0];
        if b >= b'A' && b <= b'Z' {
            return Some(letters[(b - b'A') as usize]);
        }
    }
    match name {
        "Up" => Some(Key::Up),
        "Down" => Some(Key::Down),
        "Left" => Some(Key::Left),
        "Right" => Some(Key::Right),
        "Enter" => Some(Key::Enter),
        "Space" => Some(Key::Space),
        "Tab" => Some(Key::Tab),
        "RightShift" => Some(Key::RightShift),
        "LeftShift" => Some(Key::LeftShift),
        "RightCtrl" => Some(Key::RightCtrl),
        "LeftCtrl" => Some(Key::LeftCtrl),
        _ => None,
    }
}
pub struct Bindings {
    keys: [[Key; 8]; 2],
    buttons: [[Button; 8]; 2],
}
impl Default for Bindings {
    fn default() -> Self {
        Bindings {
            keys: [
                [
                    Key::Z,
                    Key::X,
                    Key::RightShift,
                    Key::Enter,
                    Key::Up,
                    Key::Down,
                    Key::Left,
                    Key::Right,
                ],
                [
                    Key::G,
                    Key::F,
                    Key::R,
                    Key::T,
                    Key::W,
                    Key::S,
                    Key::A,
                    Key::D,
                ],
            ],
            buttons: [PAD, PAD],
        }
    }
}
impl Bindings {
    pub fn parse(text: &str) -> Result<Self, String> {
        let mut result = Self::default();
        for (number, line) in text.lines().enumerate() {
            let line = line.split('#').next().unwrap().trim();
            if line.is_empty() {
                continue;
            }
            let (name, value) = line
                .split_once('=')
                .ok_or_else(|| format!("Controls line {} needs =", number + 1))?;
            let (player, action) = name
                .trim()
                .split_once('.')
                .ok_or("Use P1.A or P2.A in controls")?;
            let index = match player {
                "P1" => 0,
                "P2" => 1,
                _ => return Err("Player must be P1 or P2".into()),
            };
            let action_index = NAMES
                .iter()
                .position(|&n| action == n || action == format!("{}Pad", n))
                .ok_or_else(|| format!("Unknown control: {}", action))?;
            if action.ends_with("Pad") {
                result.buttons[index][action_index] =
                    Button::from_string(value.trim()).ok_or("Unknown SDL gamepad button name")?;
            } else {
                result.keys[index][action_index] =
                    key(value.trim()).ok_or("Unknown keyboard key in controls")?;
            }
        }
        Ok(result)
    }
}
pub struct NativeControls {
    _sdl: Option<Sdl>,
    pump: Option<sdl2::EventPump>,
    subsystem: Option<GameControllerSubsystem>,
    pads: Vec<GameController>,
    bindings: Bindings,
}
impl NativeControls {
    pub fn new(config: Option<&Path>) -> Self {
        let bindings = config
            .and_then(|path| match ::std::fs::read_to_string(path) {
                Ok(text) => match Bindings::parse(&text) {
                    Ok(b) => Some(b),
                    Err(e) => {
                        eprintln!("Controls: {}", e);
                        None
                    }
                },
                Err(e) => {
                    eprintln!("Controls: {}", e);
                    None
                }
            })
            .unwrap_or_default();
        let sdl = sdl2::init().ok();
        let subsystem = sdl.as_ref().and_then(|s| s.game_controller().ok());
        let pump = sdl.as_ref().and_then(|s| s.event_pump().ok());
        NativeControls {
            _sdl: sdl,
            pump,
            subsystem,
            pads: Vec::new(),
            bindings,
        }
    }
    pub fn read(&mut self, window: &Window) -> [ButtonState; 2] {
        if let Some(ref mut pump) = self.pump {
            pump.pump_events();
        }
        if let Some(ref subsystem) = self.subsystem {
            subsystem.update();
            self.pads.retain(|pad| pad.attached());
            if let Ok(count) = subsystem.num_joysticks() {
                for index in 0..count {
                    if self.pads.len() >= 2 {
                        break;
                    }
                    if let Ok(pad) = subsystem.open(index) {
                        if !self
                            .pads
                            .iter()
                            .any(|existing| existing.instance_id() == pad.instance_id())
                        {
                            self.pads.push(pad);
                        }
                    }
                }
            }
        }
        let mut result = [ButtonState::default(); 2];
        for player in 0..2 {
            let mut bits = 0;
            for action in 0..8 {
                if window.is_key_down(self.bindings.keys[player][action])
                    || self.pads.get(player).map_or(false, |pad| {
                        pad.button(self.bindings.buttons[player][action])
                    })
                {
                    bits |= 1 << action;
                }
            }
            if let Some(pad) = self.pads.get(player) {
                if pad.axis(Axis::LeftX) < -16000 {
                    bits |= 64;
                }
                if pad.axis(Axis::LeftX) > 16000 {
                    bits |= 128;
                }
                if pad.axis(Axis::LeftY) < -16000 {
                    bits |= 16;
                }
                if pad.axis(Axis::LeftY) > 16000 {
                    bits |= 32;
                }
            }
            result[player] = ButtonState::from_bits(bits);
        }
        result
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parses_two_players_and_gamepad_bindings() {
        let bindings = Bindings::parse("# custom\nP1.A=Space\nP2.Start=Q\nP1.APad=x").unwrap();
        assert_eq!(bindings.keys[0][0], Key::Space);
        assert_eq!(bindings.keys[1][3], Key::Q);
        assert_eq!(bindings.buttons[0][0], Button::X);
        assert!(Bindings::parse("P3.A=Z").is_err());
        assert!(Bindings::parse("P1.A=Oops").is_err());
    }
}
