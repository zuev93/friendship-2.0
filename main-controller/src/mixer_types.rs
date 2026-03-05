#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Volume(i16);

impl Volume {
    pub fn new(raw: i16) -> Self {
        Self(raw.clamp(0, 1000))
    }

    pub fn raw(self) -> i16 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NrLevel(i16);

impl NrLevel {
    pub fn new(raw: i16) -> Self {
        Self(raw.clamp(0, 1000))
    }

    pub fn raw(self) -> i16 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EqGain(i8);

impl EqGain {
    pub fn new(raw: i8) -> Self {
        Self(raw.clamp(-12, 12))
    }

    pub fn raw(self) -> i8 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Compression(i16);

impl Compression {
    pub fn new(raw: i16) -> Self {
        Self(raw.clamp(0, 1000))
    }

    pub fn raw(self) -> i16 {
        self.0
    }
}
