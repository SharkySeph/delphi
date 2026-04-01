use egui::{Color32, Pos2, Rect, Stroke, Vec2};

use delphi_engine::SfEvent;

/// Real-time audio visualizer: waveform display and "now playing" event view.
pub struct Visualizer {
    /// Waveform sample buffer (ring buffer of recent audio samples).
    pub waveform: Vec<f32>,
    /// Spectrum magnitudes (FFT bins).
    pub spectrum: Vec<f32>,
    /// Toggle between views.
    pub mode: VisualizerMode,
    /// Current events being played (set by app each frame).
    pub playing_events: Vec<SfEvent>,
    /// Current playback tick (derived from elapsed time + tempo).
    pub current_tick: u32,
    /// Whether playback is active.
    pub is_playing: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VisualizerMode {
    NowPlaying,
    Waveform,
    Spectrum,
    Both,
}

impl Visualizer {
    pub fn new() -> Self {
        Self {
            waveform: vec![0.0; 1024],
            spectrum: vec![0.0; 512],
            mode: VisualizerMode::NowPlaying,
            playing_events: Vec::new(),
            current_tick: 0,
            is_playing: false,
        }
    }

    /// Push new audio samples for waveform display.
    pub fn push_samples(&mut self, samples: &[f32]) {
        for &s in samples {
            self.waveform.push(s);
        }
        // Keep only the last 1024 samples
        if self.waveform.len() > 1024 {
            let excess = self.waveform.len() - 1024;
            self.waveform.drain(..excess);
        }
    }

    /// Update playback state from the transport.
    pub fn update_playback(&mut self, events: &[SfEvent], elapsed_secs: f64, bpm: f64, playing: bool) {
        self.is_playing = playing;
        if playing {
            // Convert elapsed time to tick position
            let ticks_per_beat = 480.0;
            let beats_per_sec = bpm / 60.0;
            self.current_tick = (elapsed_secs * beats_per_sec * ticks_per_beat) as u32;
            self.playing_events = events.to_vec();
        } else {
            self.current_tick = 0;
        }
    }

    pub fn ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.mode, VisualizerMode::NowPlaying, "Now Playing");
            ui.selectable_value(&mut self.mode, VisualizerMode::Waveform, "Waveform");
            ui.selectable_value(&mut self.mode, VisualizerMode::Spectrum, "Spectrum");
            ui.selectable_value(&mut self.mode, VisualizerMode::Both, "Both");
        });
        ui.separator();

        match self.mode {
            VisualizerMode::NowPlaying => self.draw_now_playing(ui),
            VisualizerMode::Waveform => self.draw_waveform(ui),
            VisualizerMode::Spectrum => self.draw_spectrum(ui),
            VisualizerMode::Both => {
                let available = ui.available_height();
                ui.allocate_ui(Vec2::new(ui.available_width(), available * 0.5), |ui| {
                    self.draw_waveform(ui);
                });
                ui.separator();
                self.draw_spectrum(ui);
            }
        }
    }

    fn draw_waveform(&self, ui: &mut egui::Ui) {
        let available = ui.available_size();
        let (response, painter) = ui.allocate_painter(available, egui::Sense::hover());
        let rect = response.rect;

        // Background
        painter.rect_filled(rect, 0.0, Color32::from_rgb(25, 25, 30));

        // Center line
        let center_y = rect.center().y;
        painter.line_segment(
            [
                Pos2::new(rect.left(), center_y),
                Pos2::new(rect.right(), center_y),
            ],
            Stroke::new(0.5, Color32::from_rgb(60, 60, 65)),
        );

        // Waveform
        if self.waveform.len() >= 2 {
            let step = rect.width() / (self.waveform.len() - 1) as f32;
            let half_h = rect.height() * 0.45;
            let points: Vec<Pos2> = self
                .waveform
                .iter()
                .enumerate()
                .map(|(i, &s)| {
                    let x = rect.left() + i as f32 * step;
                    let y = center_y - s.clamp(-1.0, 1.0) * half_h;
                    Pos2::new(x, y)
                })
                .collect();

            for pair in points.windows(2) {
                painter.line_segment(
                    [pair[0], pair[1]],
                    Stroke::new(1.5, Color32::from_rgb(86, 182, 194)),
                );
            }
        }
    }

    fn draw_spectrum(&self, ui: &mut egui::Ui) {
        let available = ui.available_size();
        let (response, painter) = ui.allocate_painter(available, egui::Sense::hover());
        let rect = response.rect;

        // Background
        painter.rect_filled(rect, 0.0, Color32::from_rgb(25, 25, 30));

        if self.spectrum.is_empty() {
            return;
        }

        let bar_count = self.spectrum.len().min(128);
        let bar_width = rect.width() / bar_count as f32;

        for i in 0..bar_count {
            let magnitude = self.spectrum[i].clamp(0.0, 1.0);
            let bar_height = magnitude * rect.height() * 0.9;
            let x = rect.left() + i as f32 * bar_width;

            let bar_rect = Rect::from_min_size(
                Pos2::new(x, rect.bottom() - bar_height),
                Vec2::new(bar_width - 1.0, bar_height),
            );

            // Color gradient: teal → green → yellow → red based on magnitude
            let color = if magnitude < 0.33 {
                Color32::from_rgb(86, 182, 194)
            } else if magnitude < 0.66 {
                Color32::from_rgb(152, 195, 121)
            } else if magnitude < 0.85 {
                Color32::from_rgb(229, 192, 123)
            } else {
                Color32::from_rgb(224, 108, 117)
            };

            painter.rect_filled(bar_rect, 1.0, color);
        }
    }

    /// Draw the "Now Playing" view showing active notes at current tick.
    fn draw_now_playing(&self, ui: &mut egui::Ui) {
        if !self.is_playing || self.playing_events.is_empty() {
            ui.centered_and_justified(|ui| {
                ui.label(
                    egui::RichText::new("Press F5 to play — active notes will appear here")
                        .color(Color32::from_rgb(120, 120, 130)),
                );
            });
            return;
        }

        let tick = self.current_tick;

        // Find currently active notes
        let active: Vec<&SfEvent> = self
            .playing_events
            .iter()
            .filter(|e| e.tick <= tick && tick < e.tick + e.duration_ticks)
            .collect();

        let available = ui.available_size();
        let (response, painter) = ui.allocate_painter(available, egui::Sense::hover());
        let rect = response.rect;

        // Background
        painter.rect_filled(rect, 0.0, Color32::from_rgb(25, 25, 30));

        if active.is_empty() {
            // Draw a subtle tick counter
            painter.text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                format!("tick {}", tick),
                egui::FontId::monospace(12.0),
                Color32::from_rgb(80, 80, 90),
            );
            return;
        }

        // Draw active notes as vertical bars (like a MIDI keyboard lit up)
        // Map MIDI 0-127 to the horizontal axis
        let bar_width = rect.width() / 128.0;
        for ev in &active {
            let x = rect.left() + ev.midi_note as f32 * bar_width;
            // Height based on velocity
            let height = rect.height() * (ev.velocity as f32 / 127.0) * 0.9;

            let bar_rect = Rect::from_min_size(
                Pos2::new(x, rect.bottom() - height),
                Vec2::new(bar_width.max(3.0), height),
            );

            let color = match ev.channel {
                9 => Color32::from_rgb(224, 108, 117), // drums: red
                0 => Color32::from_rgb(86, 182, 194),  // ch 0: teal
                1 => Color32::from_rgb(229, 192, 123), // ch 1: gold
                2 => Color32::from_rgb(152, 195, 121), // ch 2: green
                3 => Color32::from_rgb(198, 120, 221), // ch 3: purple
                _ => Color32::from_rgb(97, 175, 239),  // others: blue
            };

            painter.rect_filled(bar_rect, 2.0, color);

            // Note name label for prominent notes
            if bar_width > 4.0 || active.len() <= 12 {
                let note_name = midi_to_name(ev.midi_note);
                painter.text(
                    Pos2::new(x + bar_width * 0.5, rect.bottom() - height - 10.0),
                    egui::Align2::CENTER_BOTTOM,
                    note_name,
                    egui::FontId::monospace(9.0),
                    Color32::from_rgb(200, 200, 200),
                );
            }
        }

        // Active note count
        painter.text(
            Pos2::new(rect.left() + 4.0, rect.top() + 4.0),
            egui::Align2::LEFT_TOP,
            format!("{} notes  tick {}", active.len(), tick),
            egui::FontId::monospace(10.0),
            Color32::from_rgb(150, 150, 160),
        );
    }
}

fn midi_to_name(midi: u8) -> String {
    let names = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];
    let octave = (midi as i8 / 12) - 1;
    let note = midi % 12;
    format!("{}{}", names[note as usize], octave)
}
