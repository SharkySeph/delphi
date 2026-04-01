use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;

use delphi_core::duration::Tempo;
use delphi_engine::soundfont::{play_with_soundfont, SfEvent};

use crate::studio::StudioState;

/// Playback / transport state.
pub struct TransportState {
    playing: bool,
    looping: bool,
    play_start: Option<Instant>,
    /// Elapsed seconds since play started (for display).
    elapsed_secs: f64,
    /// BPM override (when set, overrides project tempo).
    bpm_override: Option<f64>,
    /// Soundfont status message shown in transport bar.
    sf_status: String,
}

impl TransportState {
    pub fn new() -> Self {
        Self {
            playing: false,
            looping: false,
            play_start: None,
            elapsed_secs: 0.0,
            bpm_override: None,
            sf_status: String::new(),
        }
    }

    pub fn is_playing(&self) -> bool {
        self.playing
    }

    pub fn play(
        &mut self,
        studio: &StudioState,
        stop_flag: &Arc<AtomicBool>,
        sf_path: Option<&PathBuf>,
    ) {
        if self.playing {
            return;
        }

        let sf = match sf_path {
            Some(p) if p.is_file() => {
                self.sf_status = format!("SF: {}", p.file_name().unwrap_or_default().to_string_lossy());
                p.clone()
            }
            _ => {
                self.sf_status = "⚠ No SoundFont".into();
                // Still allow playback to start (engine can fall back to oscillator)
                PathBuf::new()
            }
        };

        stop_flag.store(false, Ordering::SeqCst);
        self.playing = true;
        self.play_start = Some(Instant::now());

        let stop = stop_flag.clone();
        let tempo = studio.tempo();
        let looping = self.looping;

        // TODO: collect real SfEvents from studio cells.
        // For now, play a test chord so playback actually produces sound.
        let events: Vec<SfEvent> = vec![
            SfEvent { tick: 0,   midi_note: 60, velocity: 80, duration_ticks: 480, channel: 0, program: 0 },
            SfEvent { tick: 0,   midi_note: 64, velocity: 80, duration_ticks: 480, channel: 0, program: 0 },
            SfEvent { tick: 0,   midi_note: 67, velocity: 80, duration_ticks: 480, channel: 0, program: 0 },
            SfEvent { tick: 480, midi_note: 65, velocity: 80, duration_ticks: 480, channel: 0, program: 0 },
            SfEvent { tick: 480, midi_note: 69, velocity: 80, duration_ticks: 480, channel: 0, program: 0 },
            SfEvent { tick: 480, midi_note: 72, velocity: 80, duration_ticks: 480, channel: 0, program: 0 },
        ];

        std::thread::spawn(move || {
            if sf.is_file() {
                loop {
                    let _ = play_with_soundfont(&sf, &events, &tempo, &stop);
                    if !looping || stop.load(Ordering::Relaxed) {
                        break;
                    }
                    stop.store(false, Ordering::SeqCst);
                }
            } else {
                // No soundfont — just wait. Future: use oscillator fallback.
                while !stop.load(Ordering::Relaxed) {
                    std::thread::sleep(std::time::Duration::from_millis(50));
                }
            }
        });
    }

    pub fn stop(&mut self, stop_flag: &Arc<AtomicBool>) {
        stop_flag.store(true, Ordering::SeqCst);
        self.playing = false;
        self.play_start = None;
    }

    /// Render the transport bar UI.
    pub fn ui(
        &mut self,
        ui: &mut egui::Ui,
        studio: &StudioState,
        stop_flag: &Arc<AtomicBool>,
        sf_path: Option<&PathBuf>,
    ) {
        ui.horizontal(|ui| {
            // Play / Stop
            if self.playing {
                if ui.button("⏹ Stop").clicked() {
                    self.stop(stop_flag);
                }
            } else if ui.button("▶ Play").clicked() {
                self.play(studio, stop_flag, sf_path);
            }

            ui.separator();

            // Loop toggle
            ui.toggle_value(&mut self.looping, "🔁 Loop");

            ui.separator();

            // Tempo control
            let mut bpm = self
                .bpm_override
                .unwrap_or(studio.settings.bpm);
            ui.label("BPM:");
            if ui
                .add(egui::DragValue::new(&mut bpm).range(20.0..=300.0).speed(0.5))
                .changed()
            {
                self.bpm_override = Some(bpm);
            }

            ui.separator();

            // Time display
            if let Some(start) = self.play_start {
                self.elapsed_secs = start.elapsed().as_secs_f64();
            }
            let mins = (self.elapsed_secs / 60.0) as u32;
            let secs = self.elapsed_secs % 60.0;
            ui.monospace(format!("{:02}:{:05.2}", mins, secs));

            ui.separator();

            // Key display
            ui.label(format!("Key: {}", studio.settings.key_name));

            // Time signature
            ui.label(format!(
                "{}/{}",
                studio.settings.time_sig_num, studio.settings.time_sig_den
            ));

            ui.separator();

            // SoundFont status
            if !self.sf_status.is_empty() {
                let color = if self.sf_status.starts_with('⚠') {
                    egui::Color32::from_rgb(224, 108, 117)
                } else {
                    egui::Color32::from_rgb(152, 195, 121)
                };
                ui.label(egui::RichText::new(&self.sf_status).small().color(color));
            } else if let Some(p) = sf_path {
                let name = p.file_name().unwrap_or_default().to_string_lossy();
                ui.label(
                    egui::RichText::new(format!("SF: {}", name))
                        .small()
                        .color(egui::Color32::from_rgb(152, 195, 121)),
                );
            } else {
                ui.label(
                    egui::RichText::new("No SoundFont")
                        .small()
                        .color(egui::Color32::from_rgb(224, 108, 117)),
                );
            }
        });
    }
}
