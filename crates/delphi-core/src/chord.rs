use crate::interval::Interval;
use crate::note::Note;
use std::fmt;
use std::str::FromStr;

/// Chord quality defines the intervals that make up a chord.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ChordQuality {
    Major,
    Minor,
    Diminished,
    Augmented,
    Major7,
    Minor7,
    Dominant7,
    Diminished7,
    HalfDiminished7,
    MinorMajor7,
    Augmented7,
    Sus2,
    Sus4,
    Add9,
    Major9,
    Minor9,
    Dominant9,
    Power,
}

impl ChordQuality {
    /// Return the intervals (from root) that define this chord quality.
    pub fn intervals(&self) -> Vec<Interval> {
        use Interval as I;
        match self {
            ChordQuality::Major => vec![I::UNISON, I::MAJOR_THIRD, I::PERFECT_FIFTH],
            ChordQuality::Minor => vec![I::UNISON, I::MINOR_THIRD, I::PERFECT_FIFTH],
            ChordQuality::Diminished => vec![I::UNISON, I::MINOR_THIRD, I::TRITONE],
            ChordQuality::Augmented => vec![I::UNISON, I::MAJOR_THIRD, I::MINOR_SIXTH],
            ChordQuality::Major7 => {
                vec![I::UNISON, I::MAJOR_THIRD, I::PERFECT_FIFTH, I::MAJOR_SEVENTH]
            }
            ChordQuality::Minor7 => {
                vec![I::UNISON, I::MINOR_THIRD, I::PERFECT_FIFTH, I::MINOR_SEVENTH]
            }
            ChordQuality::Dominant7 => {
                vec![I::UNISON, I::MAJOR_THIRD, I::PERFECT_FIFTH, I::MINOR_SEVENTH]
            }
            ChordQuality::Diminished7 => {
                vec![I::UNISON, I::MINOR_THIRD, I::TRITONE, I::MAJOR_SIXTH]
            }
            ChordQuality::HalfDiminished7 => {
                vec![I::UNISON, I::MINOR_THIRD, I::TRITONE, I::MINOR_SEVENTH]
            }
            ChordQuality::MinorMajor7 => {
                vec![I::UNISON, I::MINOR_THIRD, I::PERFECT_FIFTH, I::MAJOR_SEVENTH]
            }
            ChordQuality::Augmented7 => {
                vec![I::UNISON, I::MAJOR_THIRD, I::MINOR_SIXTH, I::MINOR_SEVENTH]
            }
            ChordQuality::Sus2 => vec![I::UNISON, I::MAJOR_SECOND, I::PERFECT_FIFTH],
            ChordQuality::Sus4 => vec![I::UNISON, I::PERFECT_FOURTH, I::PERFECT_FIFTH],
            ChordQuality::Add9 => {
                vec![I::UNISON, I::MAJOR_THIRD, I::PERFECT_FIFTH, I::MAJOR_NINTH]
            }
            ChordQuality::Major9 => vec![
                I::UNISON,
                I::MAJOR_THIRD,
                I::PERFECT_FIFTH,
                I::MAJOR_SEVENTH,
                I::MAJOR_NINTH,
            ],
            ChordQuality::Minor9 => vec![
                I::UNISON,
                I::MINOR_THIRD,
                I::PERFECT_FIFTH,
                I::MINOR_SEVENTH,
                I::MAJOR_NINTH,
            ],
            ChordQuality::Dominant9 => vec![
                I::UNISON,
                I::MAJOR_THIRD,
                I::PERFECT_FIFTH,
                I::MINOR_SEVENTH,
                I::MAJOR_NINTH,
            ],
            ChordQuality::Power => vec![I::UNISON, I::PERFECT_FIFTH],
        }
    }
}

/// A chord: root note + quality → resolves to concrete notes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chord {
    pub root: Note,
    pub quality: ChordQuality,
}

impl Chord {
    pub fn new(root: Note, quality: ChordQuality) -> Self {
        Self { root, quality }
    }

    /// Resolve this chord to its constituent notes.
    pub fn notes(&self) -> Vec<Note> {
        self.quality
            .intervals()
            .iter()
            .map(|interval| self.root.transpose(interval.semitones as i16))
            .collect()
    }

    /// Get MIDI note numbers for this chord.
    pub fn to_midi(&self) -> Vec<u8> {
        self.notes().iter().map(|n| n.to_midi()).collect()
    }
}

impl fmt::Display for Chord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let q = match &self.quality {
            ChordQuality::Major => "",
            ChordQuality::Minor => "m",
            ChordQuality::Diminished => "dim",
            ChordQuality::Augmented => "aug",
            ChordQuality::Major7 => "maj7",
            ChordQuality::Minor7 => "m7",
            ChordQuality::Dominant7 => "7",
            ChordQuality::Diminished7 => "dim7",
            ChordQuality::HalfDiminished7 => "m7b5",
            ChordQuality::MinorMajor7 => "mMaj7",
            ChordQuality::Augmented7 => "aug7",
            ChordQuality::Sus2 => "sus2",
            ChordQuality::Sus4 => "sus4",
            ChordQuality::Add9 => "add9",
            ChordQuality::Major9 => "maj9",
            ChordQuality::Minor9 => "m9",
            ChordQuality::Dominant9 => "9",
            ChordQuality::Power => "5",
        };
        write!(
            f,
            "{}{}{}",
            self.root.pitch_class, self.root.accidental, q
        )
    }
}

#[derive(Debug, Clone)]
pub struct ChordParseError(pub String);

impl fmt::Display for ChordParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for ChordParseError {}

impl FromStr for Chord {
    type Err = ChordParseError;

    /// Parse chord symbols like "Cmaj7", "Am", "F#m7", "Bb7", "Dm7b5", "Gsus4".
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();
        if s.is_empty() {
            return Err(ChordParseError("Empty chord string".into()));
        }

        let mut chars = s.chars().peekable();

        // Parse root pitch class
        let pitch_char = chars
            .next()
            .ok_or_else(|| ChordParseError("Empty chord string".into()))?;
        let pitch_class = pitch_char
            .to_string()
            .parse()
            .map_err(|_| ChordParseError(format!("Invalid root: {}", pitch_char)))?;

        // Parse accidental
        let accidental = match chars.peek() {
            Some('#') => {
                chars.next();
                if chars.peek() == Some(&'#') {
                    chars.next();
                    crate::note::Accidental::DoubleSharp
                } else {
                    crate::note::Accidental::Sharp
                }
            }
            Some('b') => {
                // Peek further to distinguish 'b' accidental from 'b' in quality suffix
                // like "dim" — 'b' after root letter is accidental if followed by another 'b' or a digit or quality
                chars.next();
                if chars.peek() == Some(&'b') {
                    chars.next();
                    crate::note::Accidental::DoubleFlat
                } else {
                    crate::note::Accidental::Flat
                }
            }
            _ => crate::note::Accidental::Natural,
        };

        let root = Note::new(pitch_class, accidental, 4); // Default octave 4 for chords

        // Remaining string is the quality
        let quality_str: String = chars.collect();
        let quality = parse_quality(&quality_str)?;

        Ok(Chord::new(root, quality))
    }
}

fn parse_quality(s: &str) -> Result<ChordQuality, ChordParseError> {
    match s {
        "" | "M" | "maj" | "major" => Ok(ChordQuality::Major),
        "m" | "min" | "minor" => Ok(ChordQuality::Minor),
        "dim" | "°" => Ok(ChordQuality::Diminished),
        "aug" | "+" => Ok(ChordQuality::Augmented),
        "maj7" | "M7" | "major7" => Ok(ChordQuality::Major7),
        "m7" | "min7" | "minor7" => Ok(ChordQuality::Minor7),
        "7" | "dom7" => Ok(ChordQuality::Dominant7),
        "dim7" | "°7" => Ok(ChordQuality::Diminished7),
        "m7b5" | "ø" | "ø7" => Ok(ChordQuality::HalfDiminished7),
        "mMaj7" | "mM7" | "minMaj7" => Ok(ChordQuality::MinorMajor7),
        "aug7" | "+7" => Ok(ChordQuality::Augmented7),
        "sus2" => Ok(ChordQuality::Sus2),
        "sus4" | "sus" => Ok(ChordQuality::Sus4),
        "add9" => Ok(ChordQuality::Add9),
        "maj9" | "M9" => Ok(ChordQuality::Major9),
        "m9" | "min9" => Ok(ChordQuality::Minor9),
        "9" => Ok(ChordQuality::Dominant9),
        "5" => Ok(ChordQuality::Power),
        _ => Err(ChordParseError(format!("Unknown chord quality: '{}'", s))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::note::PitchClass;

    #[test]
    fn test_cmaj7_notes() {
        let chord: Chord = "Cmaj7".parse().unwrap();
        let midi: Vec<u8> = chord.to_midi();
        assert_eq!(midi, vec![60, 64, 67, 71]); // C4 E4 G4 B4
    }

    #[test]
    fn test_am7_notes() {
        let chord: Chord = "Am7".parse().unwrap();
        let midi = chord.to_midi();
        assert_eq!(midi, vec![69, 72, 76, 79]); // A4 C5 E5 G5
    }

    #[test]
    fn test_parse_chord_qualities() {
        let _: Chord = "C".parse().unwrap();
        let _: Chord = "Cm".parse().unwrap();
        let _: Chord = "C7".parse().unwrap();
        let _: Chord = "Cdim".parse().unwrap();
        let _: Chord = "Csus4".parse().unwrap();
        let _: Chord = "F#m7".parse().unwrap();
        let _: Chord = "Bbmaj7".parse().unwrap();
        let _: Chord = "Dm7b5".parse().unwrap();
    }

    #[test]
    fn test_chord_display() {
        let chord: Chord = "Cmaj7".parse().unwrap();
        assert_eq!(chord.to_string(), "Cmaj7");

        let chord: Chord = "Am".parse().unwrap();
        assert_eq!(chord.to_string(), "Am");
    }

    #[test]
    fn test_sharp_flat_roots() {
        let chord: Chord = "F#m".parse().unwrap();
        assert_eq!(chord.root.pitch_class, PitchClass::F);
        assert_eq!(chord.root.accidental, crate::note::Accidental::Sharp);

        let chord: Chord = "Bb7".parse().unwrap();
        assert_eq!(chord.root.pitch_class, PitchClass::B);
        assert_eq!(chord.root.accidental, crate::note::Accidental::Flat);
    }
}
