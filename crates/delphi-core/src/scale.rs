use crate::interval::Interval;
use crate::note::{Note, NoteParseError, PitchClass, Accidental};
use std::fmt;
use std::str::FromStr;

/// Scale types with their interval patterns (in semitones from root).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ScaleType {
    Major,
    NaturalMinor,
    HarmonicMinor,
    MelodicMinor,
    Dorian,
    Phrygian,
    Lydian,
    Mixolydian,
    Aeolian,
    Locrian,
    MajorPentatonic,
    MinorPentatonic,
    Blues,
    WholeTone,
    Chromatic,
}

impl ScaleType {
    /// Semitone intervals from the root for each scale degree.
    pub fn intervals(&self) -> Vec<Interval> {
        let semitones: Vec<i8> = match self {
            ScaleType::Major => vec![0, 2, 4, 5, 7, 9, 11],
            ScaleType::NaturalMinor | ScaleType::Aeolian => vec![0, 2, 3, 5, 7, 8, 10],
            ScaleType::HarmonicMinor => vec![0, 2, 3, 5, 7, 8, 11],
            ScaleType::MelodicMinor => vec![0, 2, 3, 5, 7, 9, 11],
            ScaleType::Dorian => vec![0, 2, 3, 5, 7, 9, 10],
            ScaleType::Phrygian => vec![0, 1, 3, 5, 7, 8, 10],
            ScaleType::Lydian => vec![0, 2, 4, 6, 7, 9, 11],
            ScaleType::Mixolydian => vec![0, 2, 4, 5, 7, 9, 10],
            ScaleType::Locrian => vec![0, 1, 3, 5, 6, 8, 10],
            ScaleType::MajorPentatonic => vec![0, 2, 4, 7, 9],
            ScaleType::MinorPentatonic => vec![0, 3, 5, 7, 10],
            ScaleType::Blues => vec![0, 3, 5, 6, 7, 10],
            ScaleType::WholeTone => vec![0, 2, 4, 6, 8, 10],
            ScaleType::Chromatic => (0..12).collect(),
        };
        semitones.into_iter().map(Interval::new).collect()
    }
}

impl fmt::Display for ScaleType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            ScaleType::Major => "major",
            ScaleType::NaturalMinor => "minor",
            ScaleType::HarmonicMinor => "harmonic minor",
            ScaleType::MelodicMinor => "melodic minor",
            ScaleType::Dorian => "dorian",
            ScaleType::Phrygian => "phrygian",
            ScaleType::Lydian => "lydian",
            ScaleType::Mixolydian => "mixolydian",
            ScaleType::Aeolian => "aeolian",
            ScaleType::Locrian => "locrian",
            ScaleType::MajorPentatonic => "major pentatonic",
            ScaleType::MinorPentatonic => "minor pentatonic",
            ScaleType::Blues => "blues",
            ScaleType::WholeTone => "whole tone",
            ScaleType::Chromatic => "chromatic",
        };
        write!(f, "{}", s)
    }
}

/// A concrete scale: root note + scale type.
#[derive(Debug, Clone)]
pub struct Scale {
    pub root: Note,
    pub scale_type: ScaleType,
}

impl Scale {
    pub fn new(root: Note, scale_type: ScaleType) -> Self {
        Self { root, scale_type }
    }

    /// Get all notes in this scale within one octave.
    pub fn notes(&self) -> Vec<Note> {
        self.scale_type
            .intervals()
            .iter()
            .map(|i| self.root.transpose(i.semitones as i16))
            .collect()
    }

    /// Get notes spanning multiple octaves.
    pub fn notes_in_range(&self, low_midi: u8, high_midi: u8) -> Vec<Note> {
        let intervals = self.scale_type.intervals();
        let root_midi = self.root.to_midi();
        let mut result = Vec::new();

        // Go through octaves
        let start_octave = (low_midi as i16 - root_midi as i16) / 12 - 1;
        let end_octave = (high_midi as i16 - root_midi as i16) / 12 + 1;

        for oct_offset in start_octave..=end_octave {
            for interval in &intervals {
                let midi =
                    root_midi as i16 + (oct_offset * 12) + interval.semitones as i16;
                if midi >= low_midi as i16 && midi <= high_midi as i16 && midi >= 0 && midi <= 127 {
                    result.push(Note::from_midi(midi as u8));
                }
            }
        }

        result.sort_by_key(|n| n.to_midi());
        result.dedup_by_key(|n| n.to_midi());
        result
    }
}

/// A musical key: shorthand for a scale used as the tonal center.
#[derive(Debug, Clone)]
pub struct Key {
    pub root: PitchClass,
    pub accidental: Accidental,
    pub scale_type: ScaleType,
}

impl Key {
    pub fn new(root: PitchClass, accidental: Accidental, scale_type: ScaleType) -> Self {
        Self {
            root,
            accidental,
            scale_type,
        }
    }

    pub fn to_scale(&self, octave: i8) -> Scale {
        Scale::new(
            Note::new(self.root, self.accidental, octave),
            self.scale_type,
        )
    }
}

impl fmt::Display for Key {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{} {}", self.root, self.accidental, self.scale_type)
    }
}

impl FromStr for Key {
    type Err = NoteParseError;

    /// Parse key strings like "C major", "F# minor", "Bb dorian".
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();
        let parts: Vec<&str> = s.split_whitespace().collect();

        if parts.is_empty() {
            return Err(NoteParseError("Empty key string".into()));
        }

        // Parse the root (may include accidental: "C", "F#", "Bb")
        let root_str = parts[0];
        let mut chars = root_str.chars();
        let pitch_char = chars.next().ok_or_else(|| NoteParseError("Empty root".into()))?;
        let pitch_class: PitchClass = pitch_char.to_string().parse()?;

        let acc_str: String = chars.collect();
        let accidental = match acc_str.as_str() {
            "#" => Accidental::Sharp,
            "##" => Accidental::DoubleSharp,
            "b" => Accidental::Flat,
            "bb" => Accidental::DoubleFlat,
            "" => Accidental::Natural,
            _ => return Err(NoteParseError(format!("Invalid accidental: {}", acc_str))),
        };

        // Parse scale type (default to major)
        let scale_type = if parts.len() > 1 {
            match parts[1..].join(" ").to_lowercase().as_str() {
                "major" | "maj" => ScaleType::Major,
                "minor" | "min" => ScaleType::NaturalMinor,
                "harmonic minor" => ScaleType::HarmonicMinor,
                "melodic minor" => ScaleType::MelodicMinor,
                "dorian" => ScaleType::Dorian,
                "phrygian" => ScaleType::Phrygian,
                "lydian" => ScaleType::Lydian,
                "mixolydian" => ScaleType::Mixolydian,
                "aeolian" => ScaleType::Aeolian,
                "locrian" => ScaleType::Locrian,
                "pentatonic" | "major pentatonic" => ScaleType::MajorPentatonic,
                "minor pentatonic" => ScaleType::MinorPentatonic,
                "blues" => ScaleType::Blues,
                other => {
                    return Err(NoteParseError(format!("Unknown scale type: {}", other)))
                }
            }
        } else {
            ScaleType::Major
        };

        Ok(Key::new(pitch_class, accidental, scale_type))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_c_major_scale() {
        let c4 = Note::new(PitchClass::C, Accidental::Natural, 4);
        let scale = Scale::new(c4, ScaleType::Major);
        let midi: Vec<u8> = scale.notes().iter().map(|n| n.to_midi()).collect();
        assert_eq!(midi, vec![60, 62, 64, 65, 67, 69, 71]);
    }

    #[test]
    fn test_a_minor_scale() {
        let a4 = Note::new(PitchClass::A, Accidental::Natural, 4);
        let scale = Scale::new(a4, ScaleType::NaturalMinor);
        let midi: Vec<u8> = scale.notes().iter().map(|n| n.to_midi()).collect();
        assert_eq!(midi, vec![69, 71, 72, 74, 76, 77, 79]);
    }

    #[test]
    fn test_parse_key() {
        let key: Key = "C major".parse().unwrap();
        assert_eq!(key.root, PitchClass::C);
        assert_eq!(key.scale_type, ScaleType::Major);

        let key: Key = "F# minor".parse().unwrap();
        assert_eq!(key.root, PitchClass::F);
        assert_eq!(key.accidental, Accidental::Sharp);
        assert_eq!(key.scale_type, ScaleType::NaturalMinor);

        let key: Key = "Bb dorian".parse().unwrap();
        assert_eq!(key.root, PitchClass::B);
        assert_eq!(key.accidental, Accidental::Flat);
        assert_eq!(key.scale_type, ScaleType::Dorian);
    }

    #[test]
    fn test_blues_scale() {
        let c4 = Note::new(PitchClass::C, Accidental::Natural, 4);
        let scale = Scale::new(c4, ScaleType::Blues);
        let midi: Vec<u8> = scale.notes().iter().map(|n| n.to_midi()).collect();
        assert_eq!(midi, vec![60, 63, 65, 66, 67, 70]);
    }
}
