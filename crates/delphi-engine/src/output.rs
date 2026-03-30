use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use delphi_core::duration::Tempo;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};

use crate::scheduler::AudioEvent;
use crate::synth::{midi_to_freq, Envelope, Waveform};

/// An active note being rendered by the audio output.
struct ActiveNote {
    frequency: f32,
    velocity: f32,
    start_sample: u64,
    end_sample: u64,
    envelope: Envelope,
    phase: f32,
    waveform: Waveform,
}

/// Real-time audio output using cpal.
pub struct AudioOutput {
    _sample_rate: u32,
}

impl AudioOutput {
    pub fn new() -> Self {
        Self { _sample_rate: 44100 }
    }

    /// Play a list of scheduled audio events and block until playback completes.
    pub fn play_events(
        &self,
        events: &[AudioEvent],
        tempo: &Tempo,
        stop: &Arc<AtomicBool>,
    ) -> Result<(), AudioOutputError> {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or(AudioOutputError::NoDevice)?;
        let config = device
            .default_output_config()
            .map_err(|e| AudioOutputError::Config(e.to_string()))?;

        let sample_rate = config.sample_rate().0;
        let channels = config.channels() as usize;

        // Pre-compute active notes with sample positions
        let active_notes: Arc<Mutex<Vec<ActiveNote>>> = Arc::new(Mutex::new(Vec::new()));

        for event in events {
            let start_sec = event.start_seconds(tempo);
            let dur_sec = event.duration_seconds(tempo);
            let start_sample = (start_sec * sample_rate as f64) as u64;
            let end_sample = start_sample + (dur_sec * sample_rate as f64) as u64;
            let vel = event.velocity.0 as f32 / 127.0;

            active_notes.lock().unwrap().push(ActiveNote {
                frequency: midi_to_freq(event.midi_note),
                velocity: vel,
                start_sample,
                end_sample,
                envelope: Envelope::default(),
                phase: 0.0,
                waveform: Waveform::Triangle, // default timbre for MVP
            });
        }

        // Calculate total duration
        let total_samples = {
            let notes = active_notes.lock().unwrap();
            notes
                .iter()
                .map(|n| n.end_sample + (n.envelope.release * sample_rate as f32) as u64)
                .max()
                .unwrap_or(0)
        };

        let current_sample = Arc::new(Mutex::new(0u64));
        let done = Arc::new(Mutex::new(false));

        let notes_ref = Arc::clone(&active_notes);
        let sample_ref = Arc::clone(&current_sample);
        let done_ref = Arc::clone(&done);
        let sr = sample_rate as f32;

        let stream = device.build_output_stream(
            &config.into(),
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                let mut sample_pos = sample_ref.lock().unwrap();
                let mut notes = notes_ref.lock().unwrap();

                for frame in data.chunks_mut(channels) {
                    let pos = *sample_pos;
                    let mut mix = 0.0f32;

                    for note in notes.iter_mut() {
                        if pos < note.start_sample {
                            continue;
                        }

                        let time_since_on = (pos - note.start_sample) as f32 / sr;

                        let note_off_time = if pos >= note.end_sample {
                            Some((note.end_sample - note.start_sample) as f32 / sr)
                        } else {
                            None
                        };

                        let env_amp = note.envelope.amplitude(time_since_on, note_off_time);
                        if env_amp <= 0.001 {
                            continue;
                        }

                        // Generate oscillator sample
                        let osc = match note.waveform {
                            Waveform::Sine => {
                                (2.0 * std::f32::consts::PI * note.phase).sin()
                            }
                            Waveform::Triangle => {
                                if note.phase < 0.5 {
                                    4.0 * note.phase - 1.0
                                } else {
                                    3.0 - 4.0 * note.phase
                                }
                            }
                            Waveform::Square => {
                                if note.phase < 0.5 {
                                    1.0
                                } else {
                                    -1.0
                                }
                            }
                            Waveform::Sawtooth => 2.0 * note.phase - 1.0,
                        };

                        note.phase += note.frequency / sr;
                        if note.phase >= 1.0 {
                            note.phase -= 1.0;
                        }

                        mix += osc * env_amp * note.velocity * 0.3; // master gain
                    }

                    // Soft clamp
                    mix = mix.tanh();

                    for sample in frame.iter_mut() {
                        *sample = mix;
                    }

                    *sample_pos += 1;

                    if *sample_pos >= total_samples {
                        *done_ref.lock().unwrap() = true;
                    }
                }
            },
            |err| eprintln!("Audio stream error: {}", err),
            None,
        ).map_err(|e| AudioOutputError::Stream(e.to_string()))?;

        stream
            .play()
            .map_err(|e| AudioOutputError::Stream(e.to_string()))?;

        // Wait for playback to complete
        loop {
            std::thread::sleep(std::time::Duration::from_millis(50));
            if stop.load(Ordering::Relaxed) {
                break;
            }
            if *done.lock().unwrap() {
                // Allow release tails to finish
                std::thread::sleep(std::time::Duration::from_millis(200));
                break;
            }
        }

        Ok(())
    }
}

#[derive(Debug)]
pub enum AudioOutputError {
    NoDevice,
    Config(String),
    Stream(String),
}

impl std::fmt::Display for AudioOutputError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AudioOutputError::NoDevice => write!(f, "No audio output device found"),
            AudioOutputError::Config(e) => write!(f, "Audio config error: {}", e),
            AudioOutputError::Stream(e) => write!(f, "Audio stream error: {}", e),
        }
    }
}

impl std::error::Error for AudioOutputError {}
