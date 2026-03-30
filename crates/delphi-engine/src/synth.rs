use std::f32::consts::PI;

/// Waveform shapes for the built-in synthesizer.
#[derive(Debug, Clone, Copy)]
pub enum Waveform {
    Sine,
    Square,
    Sawtooth,
    Triangle,
}

/// ADSR envelope parameters (all in seconds).
#[derive(Debug, Clone, Copy)]
pub struct Envelope {
    pub attack: f32,
    pub decay: f32,
    pub sustain: f32, // level 0.0-1.0
    pub release: f32,
}

impl Default for Envelope {
    fn default() -> Self {
        Self {
            attack: 0.01,
            decay: 0.1,
            sustain: 0.7,
            release: 0.15,
        }
    }
}

impl Envelope {
    /// Get envelope amplitude at a given time since note-on, and time since note-off (if released).
    pub fn amplitude(&self, time_since_on: f32, note_off_time: Option<f32>) -> f32 {
        let ads = if time_since_on < self.attack {
            // Attack phase
            time_since_on / self.attack
        } else if time_since_on < self.attack + self.decay {
            // Decay phase
            let decay_progress = (time_since_on - self.attack) / self.decay;
            1.0 - decay_progress * (1.0 - self.sustain)
        } else {
            // Sustain phase
            self.sustain
        };

        if let Some(off_time) = note_off_time {
            let release_elapsed = time_since_on - off_time;
            if release_elapsed >= self.release {
                0.0
            } else {
                // Get amplitude at the moment of note-off
                let amp_at_off = if off_time < self.attack {
                    off_time / self.attack
                } else if off_time < self.attack + self.decay {
                    let dp = (off_time - self.attack) / self.decay;
                    1.0 - dp * (1.0 - self.sustain)
                } else {
                    self.sustain
                };
                amp_at_off * (1.0 - release_elapsed / self.release)
            }
        } else {
            ads
        }
    }
}

/// A single oscillator voice.
pub struct Oscillator {
    pub waveform: Waveform,
    pub envelope: Envelope,
    phase: f32,
    sample_rate: f32,
}

impl Oscillator {
    pub fn new(waveform: Waveform, sample_rate: f32) -> Self {
        Self {
            waveform,
            envelope: Envelope::default(),
            phase: 0.0,
            sample_rate,
        }
    }

    /// Generate the next sample for a given frequency.
    pub fn next_sample(&mut self, frequency: f32) -> f32 {
        let sample = match self.waveform {
            Waveform::Sine => (2.0 * PI * self.phase).sin(),
            Waveform::Square => {
                if self.phase < 0.5 {
                    1.0
                } else {
                    -1.0
                }
            }
            Waveform::Sawtooth => 2.0 * self.phase - 1.0,
            Waveform::Triangle => {
                if self.phase < 0.5 {
                    4.0 * self.phase - 1.0
                } else {
                    3.0 - 4.0 * self.phase
                }
            }
        };

        self.phase += frequency / self.sample_rate;
        if self.phase >= 1.0 {
            self.phase -= 1.0;
        }

        sample
    }

    pub fn reset(&mut self) {
        self.phase = 0.0;
    }
}

/// Convert a MIDI note number to frequency in Hz (A4 = 440 Hz).
pub fn midi_to_freq(midi_note: u8) -> f32 {
    440.0 * 2.0f32.powf((midi_note as f32 - 69.0) / 12.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_midi_to_freq() {
        // A4 = 440 Hz
        assert!((midi_to_freq(69) - 440.0).abs() < 0.01);
        // C4 ≈ 261.63 Hz
        assert!((midi_to_freq(60) - 261.63).abs() < 0.1);
    }

    #[test]
    fn test_oscillator_sine() {
        let mut osc = Oscillator::new(Waveform::Sine, 44100.0);
        let sample = osc.next_sample(440.0);
        // First sample should be near 0 (sin(0) = 0)
        assert!(sample.abs() < 0.1);
    }

    #[test]
    fn test_envelope_attack() {
        let env = Envelope {
            attack: 0.1,
            decay: 0.1,
            sustain: 0.7,
            release: 0.1,
        };
        // Midway through attack
        let amp = env.amplitude(0.05, None);
        assert!((amp - 0.5).abs() < 0.01);
    }
}
