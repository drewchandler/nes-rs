#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Region {
    Ntsc,
    Pal,
    Dendy,
}
impl Region {
    pub fn cpu_hz(self) -> f64 {
        match self {
            Region::Ntsc => 21_477_272.727272727 / 12.0,
            Region::Pal => 26_601_712.5 / 16.0,
            Region::Dendy => 26_601_712.5 / 15.0,
        }
    }
    pub fn scanlines(self) -> i16 {
        if self == Region::Ntsc {
            262
        } else {
            312
        }
    }
    pub fn vblank_line(self) -> i16 {
        if self == Region::Dendy {
            291
        } else {
            241
        }
    }
    pub fn fps(self) -> f64 {
        self.cpu_hz() * if self == Region::Pal { 3.2 } else { 3.0 }
            / (341.0 * self.scanlines() as f64 - if self == Region::Ntsc { 0.5 } else { 0.0 })
    }
}
