use std::fmt;

/// Musical intervals measured in semitones.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Interval {
    pub semitones: i8,
}

impl Interval {
    pub const fn new(semitones: i8) -> Self {
        Self { semitones }
    }

    // Common intervals
    pub const UNISON: Self = Self::new(0);
    pub const MINOR_SECOND: Self = Self::new(1);
    pub const MAJOR_SECOND: Self = Self::new(2);
    pub const MINOR_THIRD: Self = Self::new(3);
    pub const MAJOR_THIRD: Self = Self::new(4);
    pub const PERFECT_FOURTH: Self = Self::new(5);
    pub const TRITONE: Self = Self::new(6);
    pub const PERFECT_FIFTH: Self = Self::new(7);
    pub const MINOR_SIXTH: Self = Self::new(8);
    pub const MAJOR_SIXTH: Self = Self::new(9);
    pub const MINOR_SEVENTH: Self = Self::new(10);
    pub const MAJOR_SEVENTH: Self = Self::new(11);
    pub const OCTAVE: Self = Self::new(12);
    pub const MINOR_NINTH: Self = Self::new(13);
    pub const MAJOR_NINTH: Self = Self::new(14);
    pub const MINOR_TENTH: Self = Self::new(15);
    pub const MAJOR_TENTH: Self = Self::new(16);
    pub const PERFECT_ELEVENTH: Self = Self::new(17);
    pub const SHARP_ELEVENTH: Self = Self::new(18);
    pub const PERFECT_TWELFTH: Self = Self::new(19);
    pub const MINOR_THIRTEENTH: Self = Self::new(20);
    pub const MAJOR_THIRTEENTH: Self = Self::new(21);
}

impl fmt::Display for Interval {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self.semitones {
            0 => "unison",
            1 => "minor 2nd",
            2 => "major 2nd",
            3 => "minor 3rd",
            4 => "major 3rd",
            5 => "perfect 4th",
            6 => "tritone",
            7 => "perfect 5th",
            8 => "minor 6th",
            9 => "major 6th",
            10 => "minor 7th",
            11 => "major 7th",
            12 => "octave",
            n => return write!(f, "{} semitones", n),
        };
        write!(f, "{}", name)
    }
}
