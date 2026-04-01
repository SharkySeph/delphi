use std::fmt;

/// Duration of a note, measured in ticks (subdivisions of a quarter note).
/// Default resolution: 480 ticks per quarter note (standard MIDI PPQ).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Duration {
    pub ticks: u32,
}

impl Duration {
    pub const TICKS_PER_QUARTER: u32 = 480;

    pub const fn new(ticks: u32) -> Self {
        Self { ticks }
    }

    // Standard durations
    pub const DOUBLE_WHOLE: Self = Self::new(Self::TICKS_PER_QUARTER * 8);
    pub const WHOLE: Self = Self::new(Self::TICKS_PER_QUARTER * 4);
    pub const HALF: Self = Self::new(Self::TICKS_PER_QUARTER * 2);
    pub const QUARTER: Self = Self::new(Self::TICKS_PER_QUARTER);
    pub const EIGHTH: Self = Self::new(Self::TICKS_PER_QUARTER / 2);
    pub const SIXTEENTH: Self = Self::new(Self::TICKS_PER_QUARTER / 4);
    pub const THIRTY_SECOND: Self = Self::new(Self::TICKS_PER_QUARTER / 8);
    pub const SIXTY_FOURTH: Self = Self::new(Self::TICKS_PER_QUARTER / 16);
    pub const ONE_TWENTY_EIGHTH: Self = Self::new(Self::TICKS_PER_QUARTER / 32);

    /// Dotted duration (1.5x).
    pub const fn dotted(self) -> Self {
        Self::new(self.ticks + self.ticks / 2)
    }

    /// Double-dotted duration (1.75x).
    pub const fn double_dotted(self) -> Self {
        Self::new(self.ticks + self.ticks / 2 + self.ticks / 4)
    }

    /// Triplet duration (2/3x).
    pub const fn triplet(self) -> Self {
        Self::new(self.ticks * 2 / 3)
    }

    /// Duration in seconds at a given tempo.
    pub fn to_seconds(&self, tempo: &Tempo) -> f64 {
        let beats = self.ticks as f64 / Self::TICKS_PER_QUARTER as f64;
        beats * 60.0 / tempo.bpm
    }

    /// Parse a duration suffix like "w", "h", "q", "8", "16", "8.", "q.", "8t".
    pub fn from_suffix(s: &str) -> Option<Self> {
        match s {
            "dw" | "breve" => Some(Self::DOUBLE_WHOLE),
            "w" | "whole" => Some(Self::WHOLE),
            "h" | "half" => Some(Self::HALF),
            "q" | "quarter" => Some(Self::QUARTER),
            "8" | "eighth" => Some(Self::EIGHTH),
            "16" | "sixteenth" => Some(Self::SIXTEENTH),
            "32" => Some(Self::THIRTY_SECOND),
            "64" => Some(Self::SIXTY_FOURTH),
            "128" => Some(Self::ONE_TWENTY_EIGHTH),
            // Dotted
            "dw." => Some(Self::DOUBLE_WHOLE.dotted()),
            "w." => Some(Self::WHOLE.dotted()),
            "h." => Some(Self::HALF.dotted()),
            "q." => Some(Self::QUARTER.dotted()),
            "8." => Some(Self::EIGHTH.dotted()),
            "16." => Some(Self::SIXTEENTH.dotted()),
            "32." => Some(Self::THIRTY_SECOND.dotted()),
            // Double-dotted
            "w.." => Some(Self::WHOLE.double_dotted()),
            "h.." => Some(Self::HALF.double_dotted()),
            "q.." => Some(Self::QUARTER.double_dotted()),
            "8.." => Some(Self::EIGHTH.double_dotted()),
            // Triplet
            "wt" => Some(Self::WHOLE.triplet()),
            "ht" => Some(Self::HALF.triplet()),
            "qt" => Some(Self::QUARTER.triplet()),
            "8t" => Some(Self::EIGHTH.triplet()),
            "16t" => Some(Self::SIXTEENTH.triplet()),
            _ => None,
        }
    }
}

impl fmt::Display for Duration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.ticks {
            t if t == Self::WHOLE.ticks => write!(f, "whole"),
            t if t == Self::HALF.ticks => write!(f, "half"),
            t if t == Self::QUARTER.ticks => write!(f, "quarter"),
            t if t == Self::EIGHTH.ticks => write!(f, "eighth"),
            t if t == Self::SIXTEENTH.ticks => write!(f, "sixteenth"),
            t => write!(f, "{} ticks", t),
        }
    }
}

/// Tempo in beats per minute.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tempo {
    pub bpm: f64,
}

impl Tempo {
    pub fn new(bpm: f64) -> Self {
        Self { bpm }
    }

    /// Microseconds per MIDI quarter note (used in MIDI tempo events).
    pub fn to_midi_tempo(&self) -> u32 {
        (60_000_000.0 / self.bpm).round() as u32
    }

    /// Create from MIDI tempo (microseconds per quarter note).
    pub fn from_midi_tempo(uspqn: u32) -> Self {
        Self {
            bpm: 60_000_000.0 / uspqn as f64,
        }
    }
}

impl Default for Tempo {
    fn default() -> Self {
        Self { bpm: 120.0 }
    }
}

impl fmt::Display for Tempo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} BPM", self.bpm)
    }
}

/// Time signature.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TimeSignature {
    pub numerator: u8,
    pub denominator: u8,
}

impl TimeSignature {
    pub fn new(numerator: u8, denominator: u8) -> Self {
        Self {
            numerator,
            denominator,
        }
    }

    /// Number of ticks in one measure.
    pub fn measure_ticks(&self) -> u32 {
        let beat_ticks = Duration::TICKS_PER_QUARTER * 4 / self.denominator as u32;
        beat_ticks * self.numerator as u32
    }

    /// Standard 4/4 time.
    pub const COMMON: Self = Self {
        numerator: 4,
        denominator: 4,
    };

    /// 3/4 waltz time.
    pub const WALTZ: Self = Self {
        numerator: 3,
        denominator: 4,
    };
}

impl Default for TimeSignature {
    fn default() -> Self {
        Self::COMMON
    }
}

impl fmt::Display for TimeSignature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}", self.numerator, self.denominator)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_standard_durations() {
        assert_eq!(Duration::WHOLE.ticks, 1920);
        assert_eq!(Duration::HALF.ticks, 960);
        assert_eq!(Duration::QUARTER.ticks, 480);
        assert_eq!(Duration::EIGHTH.ticks, 240);
        assert_eq!(Duration::SIXTEENTH.ticks, 120);
    }

    #[test]
    fn test_dotted() {
        assert_eq!(Duration::QUARTER.dotted().ticks, 720);
        assert_eq!(Duration::EIGHTH.dotted().ticks, 360);
    }

    #[test]
    fn test_tempo_to_seconds() {
        let tempo = Tempo::new(120.0);
        let dur = Duration::QUARTER;
        assert!((dur.to_seconds(&tempo) - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_midi_tempo() {
        let tempo = Tempo::new(120.0);
        assert_eq!(tempo.to_midi_tempo(), 500_000);
    }

    #[test]
    fn test_time_sig_measure_ticks() {
        assert_eq!(TimeSignature::COMMON.measure_ticks(), 1920);
        assert_eq!(TimeSignature::WALTZ.measure_ticks(), 1440);
    }

    #[test]
    fn test_duration_suffix() {
        assert_eq!(Duration::from_suffix("q"), Some(Duration::QUARTER));
        assert_eq!(Duration::from_suffix("8"), Some(Duration::EIGHTH));
        assert_eq!(Duration::from_suffix("h."), Some(Duration::HALF.dotted()));
    }
}
