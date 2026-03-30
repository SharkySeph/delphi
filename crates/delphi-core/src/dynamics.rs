use std::fmt;

/// MIDI velocity (0-127). Maps to musical dynamics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Velocity(pub u8);

impl Velocity {
    pub fn new(val: u8) -> Self {
        Self(val.min(127))
    }

    pub fn from_dynamic(dynamic: Dynamic) -> Self {
        Self(dynamic.velocity())
    }
}

impl Default for Velocity {
    fn default() -> Self {
        Self(80) // mf
    }
}

impl fmt::Display for Velocity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "vel:{}", self.0)
    }
}

/// Standard musical dynamics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Dynamic {
    PPP,
    PP,
    P,
    MP,
    MF,
    F,
    FF,
    FFF,
}

impl Dynamic {
    pub fn velocity(&self) -> u8 {
        match self {
            Dynamic::PPP => 16,
            Dynamic::PP => 33,
            Dynamic::P => 49,
            Dynamic::MP => 64,
            Dynamic::MF => 80,
            Dynamic::F => 96,
            Dynamic::FF => 112,
            Dynamic::FFF => 127,
        }
    }

    pub fn from_str_dynamic(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "ppp" => Some(Dynamic::PPP),
            "pp" => Some(Dynamic::PP),
            "p" => Some(Dynamic::P),
            "mp" => Some(Dynamic::MP),
            "mf" => Some(Dynamic::MF),
            "f" => Some(Dynamic::F),
            "ff" => Some(Dynamic::FF),
            "fff" => Some(Dynamic::FFF),
            _ => None,
        }
    }
}

impl fmt::Display for Dynamic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Dynamic::PPP => "ppp",
            Dynamic::PP => "pp",
            Dynamic::P => "p",
            Dynamic::MP => "mp",
            Dynamic::MF => "mf",
            Dynamic::F => "f",
            Dynamic::FF => "ff",
            Dynamic::FFF => "fff",
        };
        write!(f, "{}", s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_velocity_clamp() {
        assert_eq!(Velocity::new(200).0, 127);
        assert_eq!(Velocity::new(64).0, 64);
    }

    #[test]
    fn test_dynamic_velocities() {
        assert!(Dynamic::PP.velocity() < Dynamic::MF.velocity());
        assert!(Dynamic::MF.velocity() < Dynamic::FF.velocity());
    }

    #[test]
    fn test_parse_dynamic() {
        assert_eq!(Dynamic::from_str_dynamic("ff"), Some(Dynamic::FF));
        assert_eq!(Dynamic::from_str_dynamic("mp"), Some(Dynamic::MP));
        assert_eq!(Dynamic::from_str_dynamic("xyz"), None);
    }
}
