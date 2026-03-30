use std::fmt;
use std::str::FromStr;

/// The 12 pitch classes (white + black keys), independent of octave.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PitchClass {
    C,
    D,
    E,
    F,
    G,
    A,
    B,
}

impl PitchClass {
    /// Semitone offset from C within an octave (C=0, D=2, ..., B=11).
    pub fn semitone_offset(self) -> i8 {
        match self {
            PitchClass::C => 0,
            PitchClass::D => 2,
            PitchClass::E => 4,
            PitchClass::F => 5,
            PitchClass::G => 7,
            PitchClass::A => 9,
            PitchClass::B => 11,
        }
    }

    /// All pitch classes in chromatic order.
    pub const ALL: [PitchClass; 7] = [
        PitchClass::C,
        PitchClass::D,
        PitchClass::E,
        PitchClass::F,
        PitchClass::G,
        PitchClass::A,
        PitchClass::B,
    ];
}

impl fmt::Display for PitchClass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            PitchClass::C => "C",
            PitchClass::D => "D",
            PitchClass::E => "E",
            PitchClass::F => "F",
            PitchClass::G => "G",
            PitchClass::A => "A",
            PitchClass::B => "B",
        };
        write!(f, "{}", s)
    }
}

impl FromStr for PitchClass {
    type Err = NoteParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "C" => Ok(PitchClass::C),
            "D" => Ok(PitchClass::D),
            "E" => Ok(PitchClass::E),
            "F" => Ok(PitchClass::F),
            "G" => Ok(PitchClass::G),
            "A" => Ok(PitchClass::A),
            "B" => Ok(PitchClass::B),
            _ => Err(NoteParseError(format!("Invalid pitch class: {}", s))),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Accidental {
    DoubleFlat,
    Flat,
    Natural,
    Sharp,
    DoubleSharp,
}

impl Accidental {
    pub fn semitone_offset(self) -> i8 {
        match self {
            Accidental::DoubleFlat => -2,
            Accidental::Flat => -1,
            Accidental::Natural => 0,
            Accidental::Sharp => 1,
            Accidental::DoubleSharp => 2,
        }
    }
}

impl fmt::Display for Accidental {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Accidental::DoubleFlat => "bb",
            Accidental::Flat => "b",
            Accidental::Natural => "",
            Accidental::Sharp => "#",
            Accidental::DoubleSharp => "##",
        };
        write!(f, "{}", s)
    }
}

/// A concrete note with pitch class, accidental, and octave.
/// Octave follows scientific pitch notation: C4 = middle C = MIDI 60.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Note {
    pub pitch_class: PitchClass,
    pub accidental: Accidental,
    pub octave: i8,
}

impl Note {
    pub fn new(pitch_class: PitchClass, accidental: Accidental, octave: i8) -> Self {
        Self {
            pitch_class,
            accidental,
            octave,
        }
    }

    /// Convert to MIDI note number (0-127). C4 = 60.
    pub fn to_midi(&self) -> u8 {
        let base = (self.octave as i16 + 1) * 12;
        let semitone =
            self.pitch_class.semitone_offset() as i16 + self.accidental.semitone_offset() as i16;
        (base + semitone).clamp(0, 127) as u8
    }

    /// Create a Note from a MIDI note number. Uses sharps for black keys.
    pub fn from_midi(midi: u8) -> Self {
        let midi = midi as i16;
        let octave = (midi / 12) - 1;
        let semitone = (midi % 12) as i8;

        let (pitch_class, accidental) = match semitone {
            0 => (PitchClass::C, Accidental::Natural),
            1 => (PitchClass::C, Accidental::Sharp),
            2 => (PitchClass::D, Accidental::Natural),
            3 => (PitchClass::D, Accidental::Sharp),
            4 => (PitchClass::E, Accidental::Natural),
            5 => (PitchClass::F, Accidental::Natural),
            6 => (PitchClass::F, Accidental::Sharp),
            7 => (PitchClass::G, Accidental::Natural),
            8 => (PitchClass::G, Accidental::Sharp),
            9 => (PitchClass::A, Accidental::Natural),
            10 => (PitchClass::A, Accidental::Sharp),
            11 => (PitchClass::B, Accidental::Natural),
            _ => unreachable!(),
        };

        Note {
            pitch_class,
            accidental,
            octave: octave as i8,
        }
    }

    /// Transpose by a number of semitones.
    pub fn transpose(&self, semitones: i16) -> Self {
        let midi = self.to_midi() as i16 + semitones;
        Self::from_midi(midi.clamp(0, 127) as u8)
    }
}

impl fmt::Display for Note {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}{}", self.pitch_class, self.accidental, self.octave)
    }
}

#[derive(Debug, Clone)]
pub struct NoteParseError(pub String);

impl fmt::Display for NoteParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for NoteParseError {}

impl FromStr for Note {
    type Err = NoteParseError;

    /// Parse a note string like "C4", "C#4", "Db5", "Ebb3".
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();
        if s.is_empty() {
            return Err(NoteParseError("Empty note string".into()));
        }

        let mut chars = s.chars();
        let pitch_char = chars
            .next()
            .ok_or_else(|| NoteParseError("Empty note string".into()))?;
        let pitch_class: PitchClass = pitch_char.to_string().parse()?;

        let rest: String = chars.collect();

        // Parse accidental prefix
        let (accidental, octave_str) = if rest.starts_with("##") {
            (Accidental::DoubleSharp, &rest[2..])
        } else if rest.starts_with('#') {
            (Accidental::Sharp, &rest[1..])
        } else if rest.starts_with("bb") {
            (Accidental::DoubleFlat, &rest[2..])
        } else if rest.starts_with('b') {
            (Accidental::Flat, &rest[1..])
        } else {
            (Accidental::Natural, rest.as_str())
        };

        let octave: i8 = octave_str
            .parse()
            .map_err(|_| NoteParseError(format!("Invalid octave in '{}'", s)))?;

        Ok(Note::new(pitch_class, accidental, octave))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_middle_c() {
        let c4 = Note::new(PitchClass::C, Accidental::Natural, 4);
        assert_eq!(c4.to_midi(), 60);
        assert_eq!(c4.to_string(), "C4");
    }

    #[test]
    fn test_parse_notes() {
        let c4: Note = "C4".parse().unwrap();
        assert_eq!(c4.to_midi(), 60);

        let cs4: Note = "C#4".parse().unwrap();
        assert_eq!(cs4.to_midi(), 61);

        let db4: Note = "Db4".parse().unwrap();
        assert_eq!(db4.to_midi(), 61);

        let a0: Note = "A0".parse().unwrap();
        assert_eq!(a0.to_midi(), 21);
    }

    #[test]
    fn test_from_midi() {
        let note = Note::from_midi(60);
        assert_eq!(note.pitch_class, PitchClass::C);
        assert_eq!(note.octave, 4);

        let note = Note::from_midi(69);
        assert_eq!(note.pitch_class, PitchClass::A);
        assert_eq!(note.octave, 4);
    }

    #[test]
    fn test_transpose() {
        let c4: Note = "C4".parse().unwrap();
        let e4 = c4.transpose(4);
        assert_eq!(e4.to_midi(), 64);
    }
}
