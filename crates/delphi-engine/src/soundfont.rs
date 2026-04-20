//! SoundFont-based audio rendering using rustysynth.
//!
//! Renders multi-track events through a SoundFont, producing realistic
//! instrument sounds via the General MIDI standard.

use std::fs::File;
use std::path::Path;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use rustysynth::{SoundFont, Synthesizer, SynthesizerSettings};

use delphi_core::duration::TempoMap;

/// Shared signal set when audio actually starts streaming.
/// The GUI can observe this to synchronize visualizations with real audio output.
pub type AudioStartSignal = Arc<Mutex<Option<Instant>>>;

/// Re-export NoteEvent as SfEvent for backward compatibility.
pub use delphi_core::event::NoteEvent as SfEvent;

/// A raw MIDI message at a specific sample position.
#[derive(Debug, Clone)]
struct TimedMessage {
    sample: u64,
    channel: i32,
    command: i32, // 0x90 = note on, 0x80 = note off, 0xC0 = program change
    data1: i32,
    data2: i32,
}

/// Render multi-voice events to the audio output using a SoundFont.
pub fn play_with_soundfont(
    sf_path: &Path,
    events: &[SfEvent],
    tempo: &TempoMap,
    stop: &Arc<AtomicBool>,
) -> Result<(), SfPlaybackError> {
    play_with_soundfont_panned(sf_path, events, tempo, stop, &[0.5; 16])
}

/// Like [`play_with_soundfont_full`] but also signals when audio actually starts.
pub fn play_with_soundfont_full_signaled(
    sf_path: &Path,
    events: &[SfEvent],
    tempo: &TempoMap,
    stop: &Arc<AtomicBool>,
    channel_pan: &[f32; 16],
    channel_reverb: &[f32; 16],
    channel_delay: &[f32; 16],
    channel_volume: &[f32; 16],
    audio_start: &AudioStartSignal,
) -> Result<(), SfPlaybackError> {
    play_with_soundfont_full_inner(sf_path, events, tempo, stop, channel_pan, channel_reverb, channel_delay, channel_volume, Some(audio_start))
}

/// Render multi-voice events to the audio output with per-channel pan (0.0=left, 0.5=center, 1.0=right).
pub fn play_with_soundfont_panned(
    sf_path: &Path,
    events: &[SfEvent],
    tempo: &TempoMap,
    stop: &Arc<AtomicBool>,
    channel_pan: &[f32; 16],
) -> Result<(), SfPlaybackError> {
    play_with_soundfont_full(sf_path, events, tempo, stop, channel_pan, &[0.0; 16], &[0.0; 16], &[1.0; 16])
}

/// Render multi-voice events to the audio output with full per-channel effects.
pub fn play_with_soundfont_full(
    sf_path: &Path,
    events: &[SfEvent],
    tempo: &TempoMap,
    stop: &Arc<AtomicBool>,
    channel_pan: &[f32; 16],
    channel_reverb: &[f32; 16],
    channel_delay: &[f32; 16],
    channel_volume: &[f32; 16],
) -> Result<(), SfPlaybackError> {
    play_with_soundfont_full_inner(sf_path, events, tempo, stop, channel_pan, channel_reverb, channel_delay, channel_volume, None)
}

fn play_with_soundfont_full_inner(
    sf_path: &Path,
    events: &[SfEvent],
    tempo: &TempoMap,
    stop: &Arc<AtomicBool>,
    channel_pan: &[f32; 16],
    channel_reverb: &[f32; 16],
    channel_delay: &[f32; 16],
    channel_volume: &[f32; 16],
    audio_start: Option<&AudioStartSignal>,
) -> Result<(), SfPlaybackError> {
    let sample_rate = 44100_u32;

    // Load the SoundFont
    let mut sf_file = File::open(sf_path)
        .map_err(|e| SfPlaybackError::SoundFont(format!("Cannot open {}: {}", sf_path.display(), e)))?;
    let sound_font = Arc::new(
        SoundFont::new(&mut sf_file)
            .map_err(|e| SfPlaybackError::SoundFont(format!("Bad SF2: {:?}", e)))?,
    );

    // Set up the synthesizer
    let settings = SynthesizerSettings::new(sample_rate as i32);
    let mut synth = Synthesizer::new(&sound_font, &settings)
        .map_err(|e| SfPlaybackError::Synth(format!("{:?}", e)))?;

    // Build timed MIDI messages from events
    let mut messages = build_messages(events, tempo, sample_rate, channel_pan, channel_reverb, channel_delay, channel_volume);
    messages.sort_by_key(|m| (m.sample, match m.command {
        0xB0 => 0, // CC (pan, etc.) first
        0xC0 => 1, // program change second
        0x80 => 2, // note-off third
        0x90 => 3, // note-on last
        c    => c, // fallback
    }));

    // Pre-render to a buffer (offline rendering for precision)
    let total_samples = messages
        .iter()
        .map(|m| m.sample)
        .max()
        .unwrap_or(0)
        + (sample_rate as u64); // 1 second tail for release

    let block_size = synth.get_block_size();
    let num_blocks = (total_samples as usize + block_size - 1) / block_size;

    let mut left_buf = vec![0.0f32; num_blocks * block_size];
    let mut right_buf = vec![0.0f32; num_blocks * block_size];

    let mut msg_idx = 0;

    for block in 0..num_blocks {
        let block_start = (block * block_size) as u64;
        let block_end = block_start + block_size as u64;

        // Process all MIDI messages in this block
        while msg_idx < messages.len() && messages[msg_idx].sample < block_end {
            let msg = &messages[msg_idx];
            synth.process_midi_message(msg.channel, msg.command, msg.data1, msg.data2);
            msg_idx += 1;
        }

        // Render one block
        let start = block * block_size;
        let end = start + block_size;
        synth.render(
            &mut left_buf[start..end],
            &mut right_buf[start..end],
        );
    }

    // Soft-clip the rendered buffer to prevent harsh digital clipping
    // when multiple instruments sum above ±1.0.
    for s in left_buf.iter_mut() {
        *s = s.tanh();
    }
    for s in right_buf.iter_mut() {
        *s = s.tanh();
    }

    // Play the rendered buffer through cpal
    play_buffer(&left_buf, &right_buf, sample_rate, stop, audio_start)?;

    Ok(())
}

/// Render multi-voice events to a WAV file using a SoundFont.
pub fn render_to_wav(
    sf_path: &Path,
    events: &[SfEvent],
    tempo: &TempoMap,
    output_path: &Path,
) -> Result<(), SfPlaybackError> {
    render_to_wav_panned(sf_path, events, tempo, output_path, &[0.5; 16])
}

/// Render multi-voice events to a WAV file with per-channel pan.
pub fn render_to_wav_panned(
    sf_path: &Path,
    events: &[SfEvent],
    tempo: &TempoMap,
    output_path: &Path,
    channel_pan: &[f32; 16],
) -> Result<(), SfPlaybackError> {
    render_to_wav_full(sf_path, events, tempo, output_path, channel_pan, &[0.0; 16], &[0.0; 16], &[1.0; 16])
}

/// Render multi-voice events to a WAV file with full per-channel effects.
pub fn render_to_wav_full(
    sf_path: &Path,
    events: &[SfEvent],
    tempo: &TempoMap,
    output_path: &Path,
    channel_pan: &[f32; 16],
    channel_reverb: &[f32; 16],
    channel_delay: &[f32; 16],
    channel_volume: &[f32; 16],
) -> Result<(), SfPlaybackError> {
    let sample_rate = 44100_u32;

    let mut sf_file = File::open(sf_path)
        .map_err(|e| SfPlaybackError::SoundFont(format!("Cannot open {}: {}", sf_path.display(), e)))?;
    let sound_font = Arc::new(
        SoundFont::new(&mut sf_file)
            .map_err(|e| SfPlaybackError::SoundFont(format!("Bad SF2: {:?}", e)))?,
    );

    let settings = SynthesizerSettings::new(sample_rate as i32);
    let mut synth = Synthesizer::new(&sound_font, &settings)
        .map_err(|e| SfPlaybackError::Synth(format!("{:?}", e)))?;

    let mut messages = build_messages(events, tempo, sample_rate, channel_pan, channel_reverb, channel_delay, channel_volume);
    messages.sort_by_key(|m| (m.sample, match m.command {
        0xB0 => 0, // CC first
        0xC0 => 1, // program change second
        0x80 => 2, // note-off third
        0x90 => 3, // note-on last
        c    => c,
    }));

    let total_samples = messages
        .iter()
        .map(|m| m.sample)
        .max()
        .unwrap_or(0)
        + (sample_rate as u64);

    let block_size = synth.get_block_size();
    let num_blocks = (total_samples as usize + block_size - 1) / block_size;

    let mut left_buf = vec![0.0f32; num_blocks * block_size];
    let mut right_buf = vec![0.0f32; num_blocks * block_size];

    let mut msg_idx = 0;

    for block in 0..num_blocks {
        let block_start = (block * block_size) as u64;
        let block_end = block_start + block_size as u64;

        while msg_idx < messages.len() && messages[msg_idx].sample < block_end {
            let msg = &messages[msg_idx];
            synth.process_midi_message(msg.channel, msg.command, msg.data1, msg.data2);
            msg_idx += 1;
        }

        let start = block * block_size;
        let end = start + block_size;
        synth.render(
            &mut left_buf[start..end],
            &mut right_buf[start..end],
        );
    }

    // Soft-clip to prevent harsh digital clipping
    for s in left_buf.iter_mut() {
        *s = s.tanh();
    }
    for s in right_buf.iter_mut() {
        *s = s.tanh();
    }

    write_wav(output_path, &left_buf, &right_buf, sample_rate)?;

    Ok(())
}

fn build_messages(events: &[SfEvent], tempo: &TempoMap, sample_rate: u32, channel_pan: &[f32; 16], channel_reverb: &[f32; 16], channel_delay: &[f32; 16], channel_volume: &[f32; 16]) -> Vec<TimedMessage> {
    let mut messages = Vec::new();

    // Emit per-channel CC messages at sample 0
    for ch in 0..16u8 {
        // CC#7 = Volume
        let vol_value = (channel_volume[ch as usize].clamp(0.0, 1.0) * 127.0).round() as i32;
        messages.push(TimedMessage {
            sample: 0,
            channel: ch as i32,
            command: 0xB0,
            data1: 7,      // CC#7 = volume
            data2: vol_value,
        });

        // CC#10 = Pan
        let pan_value = (channel_pan[ch as usize].clamp(0.0, 1.0) * 127.0).round() as i32;
        messages.push(TimedMessage {
            sample: 0,
            channel: ch as i32,
            command: 0xB0, // control change
            data1: 10,     // CC#10 = pan
            data2: pan_value,
        });

        // CC#91 = Reverb
        let reverb_value = (channel_reverb[ch as usize].clamp(0.0, 1.0) * 127.0).round() as i32;
        if reverb_value > 0 {
            messages.push(TimedMessage {
                sample: 0,
                channel: ch as i32,
                command: 0xB0,
                data1: 91,     // CC#91 = reverb
                data2: reverb_value,
            });
        }

        // CC#93 = Chorus/Delay
        let delay_value = (channel_delay[ch as usize].clamp(0.0, 1.0) * 127.0).round() as i32;
        if delay_value > 0 {
            messages.push(TimedMessage {
                sample: 0,
                channel: ch as i32,
                command: 0xB0,
                data1: 93,     // CC#93 = chorus/delay
                data2: delay_value,
            });
        }

        // CC#1 = Modulation (vibrato) — warm up solo instruments (skip drums ch9)
        if ch != 9 {
            messages.push(TimedMessage {
                sample: 0,
                channel: ch as i32,
                command: 0xB0,
                data1: 1,      // CC#1 = modulation
                data2: 40,     // moderate vibrato
            });
        }

        // CC#11 = Expression — ensure full expression level
        messages.push(TimedMessage {
            sample: 0,
            channel: ch as i32,
            command: 0xB0,
            data1: 11,     // CC#11 = expression
            data2: 127,
        });
    }

    // Sort events by tick so we can emit program changes at the right time
    let mut sorted: Vec<&SfEvent> = events.iter().collect();
    sorted.sort_by_key(|e| e.tick);

    // Track the current program per channel so we emit program changes when needed
    let mut current_program: [Option<u8>; 16] = [None; 16];

    for evt in &sorted {
        let ch = evt.channel as usize;
        let start_sec = evt.start_seconds(tempo);
        let start_sample = (start_sec * sample_rate as f64) as u64;

        // Emit program change if this channel hasn't been set yet or program differs
        if ch < 16 {
            let needs_change = match current_program[ch] {
                None => true,
                Some(p) => p != evt.program,
            };
            if needs_change {
                current_program[ch] = Some(evt.program);
                messages.push(TimedMessage {
                    sample: start_sample,
                    channel: evt.channel as i32,
                    command: 0xC0, // program change
                    data1: evt.program as i32,
                    data2: 0,
                });
            }
        }

        let dur_sec = evt.duration_seconds(tempo);
        let end_sample = start_sample + (dur_sec * sample_rate as f64) as u64;

        messages.push(TimedMessage {
            sample: start_sample,
            channel: evt.channel as i32,
            command: 0x90, // note on
            data1: evt.midi_note as i32,
            data2: evt.velocity as i32,
        });

        messages.push(TimedMessage {
            sample: end_sample,
            channel: evt.channel as i32,
            command: 0x80, // note off
            data1: evt.midi_note as i32,
            data2: 0,
        });
    }

    messages
}

fn play_buffer(left: &[f32], right: &[f32], sample_rate: u32, stop: &Arc<AtomicBool>, audio_start: Option<&AudioStartSignal>) -> Result<(), SfPlaybackError> {
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .ok_or(SfPlaybackError::Audio("No audio output device found".into()))?;

    let config = cpal::StreamConfig {
        channels: 2,
        sample_rate: cpal::SampleRate(sample_rate),
        buffer_size: cpal::BufferSize::Default,
    };

    let left = left.to_vec();
    let right = right.to_vec();
    let total_frames = left.len();

    let pos = std::sync::Arc::new(std::sync::Mutex::new(0usize));
    let done = std::sync::Arc::new(std::sync::Mutex::new(false));

    let pos_ref = std::sync::Arc::clone(&pos);
    let done_ref = std::sync::Arc::clone(&done);

    let stream = device
        .build_output_stream(
            &config,
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                let mut p = pos_ref.lock().unwrap();
                for frame in data.chunks_mut(2) {
                    if *p < total_frames {
                        frame[0] = left[*p];
                        frame[1] = right[*p];
                        *p += 1;
                    } else {
                        frame[0] = 0.0;
                        frame[1] = 0.0;
                        *done_ref.lock().unwrap() = true;
                    }
                }
            },
            |err| eprintln!("Audio stream error: {}", err),
            None,
        )
        .map_err(|e| SfPlaybackError::Audio(e.to_string()))?;

    stream
        .play()
        .map_err(|e| SfPlaybackError::Audio(e.to_string()))?;

    // Signal that audio is now actually streaming
    if let Some(signal) = audio_start {
        *signal.lock().unwrap() = Some(Instant::now());
    }

    loop {
        std::thread::sleep(std::time::Duration::from_millis(50));
        if stop.load(Ordering::Relaxed) {
            break;
        }
        if *done.lock().unwrap() {
            std::thread::sleep(std::time::Duration::from_millis(100));
            break;
        }
    }

    Ok(())
}

fn write_wav(path: &Path, left: &[f32], right: &[f32], sample_rate: u32) -> Result<(), SfPlaybackError> {
    let num_samples = left.len();
    let num_channels: u16 = 2;
    let bits_per_sample: u16 = 16;
    let byte_rate = sample_rate * num_channels as u32 * bits_per_sample as u32 / 8;
    let block_align = num_channels * bits_per_sample / 8;
    let data_size = num_samples as u32 * num_channels as u32 * (bits_per_sample as u32 / 8);
    let file_size = 36 + data_size;

    let mut buf = Vec::with_capacity(file_size as usize + 8);

    // RIFF header
    buf.extend_from_slice(b"RIFF");
    buf.extend_from_slice(&file_size.to_le_bytes());
    buf.extend_from_slice(b"WAVE");

    // fmt chunk
    buf.extend_from_slice(b"fmt ");
    buf.extend_from_slice(&16u32.to_le_bytes()); // chunk size
    buf.extend_from_slice(&1u16.to_le_bytes()); // PCM
    buf.extend_from_slice(&num_channels.to_le_bytes());
    buf.extend_from_slice(&sample_rate.to_le_bytes());
    buf.extend_from_slice(&byte_rate.to_le_bytes());
    buf.extend_from_slice(&block_align.to_le_bytes());
    buf.extend_from_slice(&bits_per_sample.to_le_bytes());

    // data chunk
    buf.extend_from_slice(b"data");
    buf.extend_from_slice(&data_size.to_le_bytes());

    for i in 0..num_samples {
        let l = (left[i].clamp(-1.0, 1.0) * 32767.0) as i16;
        let r = (right[i].clamp(-1.0, 1.0) * 32767.0) as i16;
        buf.extend_from_slice(&l.to_le_bytes());
        buf.extend_from_slice(&r.to_le_bytes());
    }

    std::fs::write(path, &buf)
        .map_err(|e| SfPlaybackError::Audio(format!("Cannot write {}: {}", path.display(), e)))?;

    Ok(())
}

#[derive(Debug)]
pub enum SfPlaybackError {
    SoundFont(String),
    Synth(String),
    Audio(String),
}

impl std::fmt::Display for SfPlaybackError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SfPlaybackError::SoundFont(e) => write!(f, "SoundFont error: {}", e),
            SfPlaybackError::Synth(e) => write!(f, "Synthesizer error: {}", e),
            SfPlaybackError::Audio(e) => write!(f, "Audio error: {}", e),
        }
    }
}

impl std::error::Error for SfPlaybackError {}

#[cfg(test)]
mod tests {
    use super::*;
    use delphi_core::duration::Tempo;

    #[test]
    fn test_build_messages() {
        let tempo = TempoMap::constant(&Tempo::new(120.0));
        let events = vec![
            SfEvent {
                tick: 0,
                midi_note: 60,
                velocity: 80,
                duration_ticks: 480,
                channel: 0,
                program: 0,
            },
            SfEvent {
                tick: 480,
                midi_note: 64,
                velocity: 80,
                duration_ticks: 480,
                channel: 1,
                program: 40,
            },
        ];

        let pan = [0.5_f32; 16];
        let reverb = [0.0_f32; 16];
        let delay = [0.0_f32; 16];
        let volume = [1.0_f32; 16];
        let messages = build_messages(&events, &tempo, 44100, &pan, &reverb, &delay, &volume);
        // Should have: 16 pan + 16 volume + 15 modulation (skip ch9) + 16 expression
        //   + 2 program changes + 2 note-on + 2 note-off = 69
        assert_eq!(messages.len(), 69);

        // Program changes
        let pc: Vec<_> = messages.iter().filter(|m| m.command == 0xC0).collect();
        assert_eq!(pc.len(), 2);
        assert_eq!(pc[0].data1, 0);  // piano
        assert_eq!(pc[1].data1, 40); // violin
    }
}
