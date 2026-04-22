use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use delphi_core::duration::{Duration, Tempo, TempoMap};
use delphi_core::dynamics::Velocity;
use delphi_engine::scheduler::AudioEvent;
use delphi_engine::soundfont::{play_with_soundfont_full_signaled, AudioStartSignal, SfEvent};
use delphi_engine::AudioOutput;

use crate::studio::StudioState;

/// Playback / transport state.
pub struct TransportState {
    playing: bool,
    looping: bool,
    play_start: Option<Instant>,
    /// Elapsed seconds since play started (for display).
    elapsed_secs: f64,
    /// BPM override (when set, overrides project tempo).
    pub bpm_override: Option<f64>,
    /// Start offset in bars (0 = beginning).
    pub start_bar: u32,
    /// Soundfont status message shown in transport bar.
    sf_status: String,
    /// Shared flag set by playback thread when it finishes.
    done_flag: Arc<AtomicBool>,
    /// Signal set by the audio thread when streaming actually begins.
    /// `elapsed_secs()` counts from this instant instead of thread-spawn time.
    audio_start: AudioStartSignal,
}

impl TransportState {
    pub fn new() -> Self {
        Self {
            playing: false,
            looping: false,
            play_start: None,
            elapsed_secs: 0.0,
            bpm_override: None,
            start_bar: 0,
            sf_status: String::new(),
            done_flag: Arc::new(AtomicBool::new(false)),
            audio_start: Arc::new(Mutex::new(None)),
        }
    }

    /// Check if the playback thread has finished and auto-stop if so.
    pub fn poll_done(&mut self) {
        if self.playing && self.done_flag.load(Ordering::Relaxed) {
            self.playing = false;
            // Snapshot final elapsed from the audio-start instant if available.
            if let Some(start) = *self.audio_start.lock().unwrap() {
                self.elapsed_secs = start.elapsed().as_secs_f64();
            } else if let Some(start) = self.play_start.take() {
                self.elapsed_secs = start.elapsed().as_secs_f64();
            }
            self.play_start = None;
            *self.audio_start.lock().unwrap() = None;
        }
    }

    pub fn is_playing(&self) -> bool {
        self.playing
    }

    pub fn elapsed_secs(&self) -> f64 {
        // Prefer the real audio-start instant (set when cpal stream begins).
        // Fall back to play_start (thread-spawn time) while the engine is
        // still loading / pre-rendering.
        if let Some(start) = *self.audio_start.lock().unwrap() {
            start.elapsed().as_secs_f64()
        } else if let Some(start) = self.play_start {
            // Audio hasn't started yet — return 0 so visualizations don't
            // race ahead during the pre-render phase.
            let _ = start;
            0.0
        } else {
            self.elapsed_secs
        }
    }

    pub fn play(
        &mut self,
        studio: &StudioState,
        stop_flag: &Arc<AtomicBool>,
        sf_path: Option<&PathBuf>,
        master_gain: f32,
    ) {
        let tempo_map = self.effective_tempo_map(studio);
        let pan = studio.channel_pan_map();
        let reverb = studio.channel_reverb_map();
        let delay = studio.channel_delay_map();
        let volume = studio.channel_volume_map();
        self.play_events(studio.collect_events_mixed(None, master_gain), tempo_map, stop_flag, sf_path, pan, reverb, delay, volume);
    }

    /// Play a single cell by index.
    pub fn play_cell(
        &mut self,
        studio: &StudioState,
        cell_idx: usize,
        stop_flag: &Arc<AtomicBool>,
        sf_path: Option<&PathBuf>,
        master_gain: f32,
    ) {
        let tempo_map = self.effective_tempo_map(studio);
        let pan = studio.channel_pan_map();
        let reverb = studio.channel_reverb_map();
        let delay = studio.channel_delay_map();
        let volume = studio.channel_volume_map();
        self.play_events(studio.collect_events_mixed(Some(cell_idx), master_gain), tempo_map, stop_flag, sf_path, pan, reverb, delay, volume);
    }

    /// Resolve effective tempo map: bpm_override if set, otherwise project tempo map.
    fn effective_tempo_map(&self, studio: &StudioState) -> TempoMap {
        match self.bpm_override {
            Some(bpm) => TempoMap::constant(&Tempo { bpm }),
            None => studio.tempo_map(),
        }
    }

    fn play_events(
        &mut self,
        mut events: Vec<SfEvent>,
        tempo_map: TempoMap,
        stop_flag: &Arc<AtomicBool>,
        sf_path: Option<&PathBuf>,
        channel_pan: [f32; 16],
        channel_reverb: [f32; 16],
        channel_delay: [f32; 16],
        channel_volume: [f32; 16],
    ) {
        if self.playing {
            return;
        }

        // Apply start-bar offset: skip events before the offset and shift remaining
        if self.start_bar > 0 {
            let ticks_per_bar = 480 * 4; // Assuming 4/4 for now
            let offset_ticks = self.start_bar * ticks_per_bar;
            events.retain(|ev| ev.tick + ev.duration_ticks > offset_ticks);
            for ev in &mut events {
                ev.tick = ev.tick.saturating_sub(offset_ticks);
            }
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
        self.done_flag.store(false, Ordering::SeqCst);
        *self.audio_start.lock().unwrap() = None;
        self.playing = true;
        self.play_start = Some(Instant::now());

        let stop = stop_flag.clone();
        let done = self.done_flag.clone();
        let audio_start = self.audio_start.clone();
        let looping = self.looping;

        std::thread::spawn(move || {
            if sf.is_file() {
                loop {
                    let _ = play_with_soundfont_full_signaled(&sf, &events, &tempo_map, &stop, &channel_pan, &channel_reverb, &channel_delay, &channel_volume, &audio_start);
                    if !looping || stop.load(Ordering::Relaxed) {
                        break;
                    }
                    // Reset audio_start for the next loop iteration
                    *audio_start.lock().unwrap() = None;
                    stop.store(false, Ordering::SeqCst);
                }
            } else {
                // Oscillator fallback: convert SfEvents to AudioEvents and play via synth
                let audio_events: Vec<AudioEvent> = events
                    .iter()
                    .map(|ev| AudioEvent {
                        tick: ev.tick,
                        midi_note: ev.midi_note,
                        velocity: Velocity(ev.velocity),
                        duration: Duration::new(ev.duration_ticks),
                    })
                    .collect();
                let output = AudioOutput::new();
                let _ = output.play_events(&audio_events, &tempo_map, &stop);
            }
            done.store(true, Ordering::SeqCst);
        });
    }

    pub fn stop(&mut self, stop_flag: &Arc<AtomicBool>) {
        stop_flag.store(true, Ordering::SeqCst);
        self.playing = false;
        self.play_start = None;
        *self.audio_start.lock().unwrap() = None;
    }

    /// Render the transport bar UI.
    pub fn ui(
        &mut self,
        ui: &mut egui::Ui,
        studio: &StudioState,
        stop_flag: &Arc<AtomicBool>,
        sf_path: Option<&PathBuf>,
        missing_project_sf: Option<&Path>,
        master_gain: f32,
    ) {
        ui.horizontal(|ui| {
            // Play / Stop
            if self.playing {
                if ui.button("⏹ Stop").clicked() {
                    self.stop(stop_flag);
                }
            } else if ui.button("▶ Play").clicked() {
                self.play(studio, stop_flag, sf_path, master_gain);
            }

            ui.separator();

            // Loop toggle
            ui.toggle_value(&mut self.looping, "🔁 Loop");

            ui.separator();

            // Start bar offset
            ui.label("Bar:");
            let mut bar = self.start_bar as i32;
            if ui
                .add(egui::DragValue::new(&mut bar).range(0..=999).speed(0.3))
                .on_hover_text("Start playback from this bar (0 = beginning)")
                .changed()
            {
                self.start_bar = bar.max(0) as u32;
            }

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
                ui.label(egui::RichText::new(&self.sf_status).small().color(color))
                    .on_hover_text("SoundFont used for playback and WAV export. Configure in the SoundFonts panel.");
            } else if let Some(path) = missing_project_sf {
                ui.label(
                    egui::RichText::new("⚠ Project SoundFont missing")
                        .small()
                        .color(egui::Color32::from_rgb(224, 108, 117)),
                )
                .on_hover_text(format!(
                    "Project SoundFont not found: {}\nPlayback uses the oscillator fallback. Open the SoundFonts panel to install, browse, or clear the missing reference.",
                    path.display()
                ));
            } else if let Some(p) = sf_path {
                if p.is_file() {
                    let name = p.file_name().unwrap_or_default().to_string_lossy();
                    ui.label(
                        egui::RichText::new(format!("SF: {}", name))
                            .small()
                            .color(egui::Color32::from_rgb(152, 195, 121)),
                    )
                    .on_hover_text(format!(
                        "SoundFont: {}\nUsed for playback and WAV export. Configure in the SoundFonts panel.",
                        p.display()
                    ));
                } else {
                    ui.label(
                        egui::RichText::new("⚠ No SoundFont")
                            .small()
                            .color(egui::Color32::from_rgb(224, 108, 117)),
                    )
                    .on_hover_text(
                        "No SoundFont loaded. Playback will use the oscillator fallback.\n\
                         Open the SoundFonts panel to add one.",
                    );
                }
            } else {
                ui.label(
                    egui::RichText::new("⚠ No SoundFont")
                        .small()
                        .color(egui::Color32::from_rgb(224, 108, 117)),
                )
                .on_hover_text(
                    "No SoundFont loaded. Playback will use the oscillator fallback.\n\
                     Open the SoundFonts panel to add one.",
                );
            }

            // Master gain display in transport bar
            ui.separator();
            ui.label(
                egui::RichText::new(format!("Vol: {:.0}%", master_gain * 100.0))
                    .small()
                    .color(egui::Color32::from_rgb(150, 150, 160)),
            )
            .on_hover_text("Master output gain (adjust in the Mixer panel)");
        });
    }
}
