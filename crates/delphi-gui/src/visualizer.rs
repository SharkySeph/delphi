use egui::{Color32, Pos2, Rect, Stroke, Vec2};

use delphi_core::duration::TempoMap;
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
    /// Last elapsed seconds seen (for incremental tick accumulation).
    last_elapsed: f64,
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
            last_elapsed: 0.0,
        }
    }

    /// Push new audio samples for waveform display.
    #[allow(dead_code)]
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
    pub fn update_playback(&mut self, events: &[SfEvent], elapsed_secs: f64, tempo_map: &TempoMap, playing: bool) {
        self.is_playing = playing;
        if playing {
            // Use the tempo map to convert elapsed time to the current tick
            self.current_tick = tempo_map.seconds_to_tick(elapsed_secs);
            self.last_elapsed = elapsed_secs;
            self.playing_events = events.to_vec();

            let bpm = tempo_map.bpm_at_tick(self.current_tick);

            // Generate synthetic waveform from active notes
            self.generate_waveform_from_events(events, elapsed_secs, bpm);
            // Generate spectrum from active notes
            self.generate_spectrum_from_events(events, elapsed_secs, bpm);
        } else {
            self.current_tick = 0;
            self.last_elapsed = 0.0;
            // Clear synthetic buffers so they don't show stale data when idle.
            for s in &mut self.waveform { *s = 0.0; }
            for b in &mut self.spectrum { *b = 0.0; }
        }
    }

    /// Synthesize a waveform visualization from active MIDI events.
    fn generate_waveform_from_events(&mut self, events: &[SfEvent], elapsed_secs: f64, _bpm: f64) {
        let tick = self.current_tick;
        let active: Vec<&SfEvent> = events
            .iter()
            .filter(|e| e.tick <= tick && tick < e.tick + e.duration_ticks)
            .collect();

        let sample_rate = 44100.0_f64;
        let num_samples = self.waveform.len();

        for i in 0..num_samples {
            let t = elapsed_secs + (i as f64 / sample_rate);
            let mut sample = 0.0_f32;
            for ev in &active {
                let freq = 440.0 * 2.0_f64.powf((ev.midi_note as f64 - 69.0) / 12.0);
                let vel = ev.velocity as f32 / 127.0;
                sample += (2.0 * std::f32::consts::PI * freq as f32 * t as f32).sin() * vel * 0.3;
            }
            self.waveform[i] = sample.tanh();
        }
    }

    /// Generate spectrum bins from active notes (frequency-domain approximation).
    fn generate_spectrum_from_events(&mut self, events: &[SfEvent], _elapsed_secs: f64, _bpm: f64) {
        let tick = self.current_tick;
        let active: Vec<&SfEvent> = events
            .iter()
            .filter(|e| e.tick <= tick && tick < e.tick + e.duration_ticks)
            .collect();

        // Clear spectrum
        for bin in self.spectrum.iter_mut() {
            *bin = 0.0;
        }

        let num_bins = self.spectrum.len();
        // Map MIDI notes to frequency bins (logarithmic, 20 Hz to ~4000 Hz across bins)
        let min_freq: f64 = 20.0;
        let max_freq: f64 = 4000.0;
        for ev in &active {
            let freq = 440.0 * 2.0_f64.powf((ev.midi_note as f64 - 69.0) / 12.0);
            if freq < min_freq || freq > max_freq {
                continue;
            }
            // Logarithmic bin mapping
            let log_pos = (freq / min_freq).ln() / (max_freq / min_freq).ln();
            let bin = (log_pos * num_bins as f64) as usize;
            if bin < num_bins {
                let vel = ev.velocity as f32 / 127.0;
                // Main frequency + harmonics spread
                self.spectrum[bin] = (self.spectrum[bin] + vel * 0.8).min(1.0);
                // Add some spread to neighboring bins
                if bin > 0 {
                    self.spectrum[bin - 1] = (self.spectrum[bin - 1] + vel * 0.3).min(1.0);
                }
                if bin + 1 < num_bins {
                    self.spectrum[bin + 1] = (self.spectrum[bin + 1] + vel * 0.3).min(1.0);
                }
            }
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
                let available_h = ui.available_height();
                let available_w = ui.available_width();
                ui.allocate_ui(Vec2::new(available_w, (available_h * 0.5).max(60.0)), |ui| {
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

        // Idle overlay when not playing
        if !self.is_playing {
            painter.text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                "Press F5 to play",
                egui::FontId::proportional(12.0),
                Color32::from_rgba_premultiplied(120, 120, 130, 160),
            );
        }
    }

    fn draw_spectrum(&self, ui: &mut egui::Ui) {
        let available = ui.available_size();
        let (response, painter) = ui.allocate_painter(available, egui::Sense::hover());
        let rect = response.rect;

        // Background
        painter.rect_filled(rect, 0.0, Color32::from_rgb(25, 25, 30));

        if !self.is_playing {
            painter.text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                "Press F5 to play",
                egui::FontId::proportional(12.0),
                Color32::from_rgba_premultiplied(120, 120, 130, 160),
            );
            return;
        }

        // Check whether any note is currently active so we can show an
        // informative label when nothing is sounding mid-playback.
        let any_active = self.spectrum.iter().any(|&v| v > 0.001);
        if !any_active {
            painter.text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                "No active notes",
                egui::FontId::proportional(12.0),
                Color32::from_rgba_premultiplied(120, 120, 130, 160),
            );
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

    /// Draw the "Now Playing" view — Strudel-style per-track lane visualization.
    /// Each track/channel gets its own horizontal lane.  Notes are colored blocks
    /// that scroll past a centered playhead.  Active notes are highlighted.
    fn draw_now_playing(&self, ui: &mut egui::Ui) {
        if !self.is_playing || self.playing_events.is_empty() {
            ui.centered_and_justified(|ui| {
                ui.label(
                    egui::RichText::new("Press F5 to play — the timeline will appear here")
                        .color(Color32::from_rgb(120, 120, 130)),
                );
            });
            return;
        }

        let tick = self.current_tick;

        let available = ui.available_size();
        let (response, painter) = ui.allocate_painter(available, egui::Sense::hover());
        let rect = response.rect;

        // Background
        painter.rect_filled(rect, 0.0, Color32::from_rgb(20, 21, 25));

        if self.playing_events.is_empty() {
            return;
        }

        // ── Collect unique channels (tracks) ──────────────────────
        let mut channels: Vec<u8> = self.playing_events.iter().map(|e| e.channel).collect();
        channels.sort();
        channels.dedup();
        let num_lanes = channels.len().max(1);

        // Channel → display name
        let channel_name = |ch: u8| -> &str {
            match ch {
                9 => "Drums",
                0 => "Piano",
                1 => "Bass",
                2 => "Melody",
                3 => "Pad",
                4 => "Lead",
                5 => "Keys",
                _ => "Track",
            }
        };

        // Channel → color (Strudel-like palette)
        let channel_color = |ch: u8| -> Color32 {
            match ch {
                0 => Color32::from_rgb(86, 182, 194),   // teal
                1 => Color32::from_rgb(229, 192, 123),   // gold
                2 => Color32::from_rgb(152, 195, 121),   // green
                3 => Color32::from_rgb(198, 120, 221),   // purple
                4 => Color32::from_rgb(97, 175, 239),    // blue
                5 => Color32::from_rgb(209, 154, 102),   // orange
                9 => Color32::from_rgb(224, 108, 117),   // red (drums)
                _ => Color32::from_rgb(150, 150, 170),
            }
        };

        // ── Timeline parameters ──────────────────────────────────
        let ticks_per_beat = 480.0_f32;
        let ticks_per_measure = ticks_per_beat * 4.0; // assume 4/4
        let visible_measures = 4.0_f32; // show 4 measures like Strudel
        let visible_ticks = visible_measures * ticks_per_measure;
        let playhead_frac = 0.5_f32; // playhead in center
        let playhead_x = rect.left() + rect.width() * playhead_frac;

        let tick_at_left = tick as f32 - visible_ticks * playhead_frac;
        let tick_at_right = tick_at_left + visible_ticks;
        let pixels_per_tick = rect.width() / visible_ticks;

        // ── Lane geometry ────────────────────────────────────────
        let header_h = 16.0_f32; // top bar for measure numbers
        let lane_gap = 2.0_f32;
        let total_lane_space = rect.height() - header_h - lane_gap * (num_lanes as f32 - 1.0).max(0.0);
        let lane_h = (total_lane_space / num_lanes as f32).clamp(20.0, 80.0);

        // ── Draw measure grid ────────────────────────────────────
        let first_measure = (tick_at_left / ticks_per_measure).floor() as i32;
        let last_measure = (tick_at_right / ticks_per_measure).ceil() as i32;
        for m in first_measure..=last_measure {
            let m_tick = m as f32 * ticks_per_measure;
            let x = rect.left() + (m_tick - tick_at_left) * pixels_per_tick;
            if x >= rect.left() && x <= rect.right() {
                // Measure bar line
                painter.line_segment(
                    [Pos2::new(x, rect.top() + header_h), Pos2::new(x, rect.bottom())],
                    Stroke::new(0.8, Color32::from_rgb(50, 52, 60)),
                );
                // Measure number
                painter.text(
                    Pos2::new(x + 4.0, rect.top() + 2.0),
                    egui::Align2::LEFT_TOP,
                    format!("{}", m + 1),
                    egui::FontId::monospace(10.0),
                    Color32::from_rgb(90, 92, 105),
                );
            }
            // Beat sub-lines
            for b in 1..4 {
                let b_tick = m_tick + b as f32 * ticks_per_beat;
                let bx = rect.left() + (b_tick - tick_at_left) * pixels_per_tick;
                if bx >= rect.left() && bx <= rect.right() {
                    painter.line_segment(
                        [Pos2::new(bx, rect.top() + header_h), Pos2::new(bx, rect.bottom())],
                        Stroke::new(0.3, Color32::from_rgb(38, 40, 48)),
                    );
                }
            }
        }

        // ── Draw lanes ──────────────────────────────────────────
        for (lane_idx, &ch) in channels.iter().enumerate() {
            let lane_top = rect.top() + header_h + lane_idx as f32 * (lane_h + lane_gap);
            let lane_rect = Rect::from_min_size(
                Pos2::new(rect.left(), lane_top),
                Vec2::new(rect.width(), lane_h),
            );

            // Lane background (subtle alternating)
            let bg = if lane_idx % 2 == 0 {
                Color32::from_rgb(26, 27, 32)
            } else {
                Color32::from_rgb(30, 31, 36)
            };
            painter.rect_filled(lane_rect, 0.0, bg);

            // Lane label on the left
            painter.text(
                Pos2::new(rect.left() + 4.0, lane_top + lane_h * 0.5),
                egui::Align2::LEFT_CENTER,
                channel_name(ch),
                egui::FontId::monospace(9.0),
                Color32::from_rgb(80, 82, 95),
            );

            // ── Draw note blocks for this channel ────────────────
            // Find pitch range within this channel for vertical mapping
            let ch_events: Vec<&SfEvent> = self.playing_events.iter()
                .filter(|e| e.channel == ch)
                .collect();

            let (min_note, max_note) = if ch == 9 {
                // Drums: use fixed range, fold all into lane
                (35u8, 81u8)
            } else {
                let mn = ch_events.iter().map(|e| e.midi_note).min().unwrap_or(60);
                let mx = ch_events.iter().map(|e| e.midi_note).max().unwrap_or(72);
                (mn.saturating_sub(1), mx + 1)
            };
            let note_range = (max_note - min_note).max(1) as f32;

            let pad_y = 3.0_f32; // vertical padding inside lane
            let inner_h = lane_h - pad_y * 2.0;

            for ev in &ch_events {
                let ev_start = ev.tick as f32;
                let ev_end = ev_start + ev.duration_ticks as f32;

                // Skip if outside viewport
                if ev_end < tick_at_left || ev_start > tick_at_right {
                    continue;
                }

                let x1 = (rect.left() + (ev_start - tick_at_left) * pixels_per_tick).max(rect.left());
                let x2 = (rect.left() + (ev_end - tick_at_left) * pixels_per_tick).min(rect.right());

                // Vertical position within lane based on pitch
                let note_frac = (ev.midi_note - min_note) as f32 / note_range;
                let block_h = (inner_h / note_range).clamp(3.0, inner_h * 0.8);
                let y_center = lane_top + pad_y + inner_h * (1.0 - note_frac);

                let is_active = ev.tick <= tick && tick < ev.tick + ev.duration_ticks;
                let base = channel_color(ch);

                let color = if is_active {
                    // Bright / highlighted (Strudel active: warm gold glow)
                    Color32::from_rgb(
                        (base.r() as u16 + 40).min(255) as u8,
                        (base.g() as u16 + 40).min(255) as u8,
                        (base.b() as u16 + 40).min(255) as u8,
                    )
                } else {
                    // Inactive: softly colored (Strudel style — visible but muted)
                    Color32::from_rgba_premultiplied(
                        base.r() / 2 + 20,
                        base.g() / 2 + 20,
                        base.b() / 2 + 20,
                        160,
                    )
                };

                let block_rect = Rect::from_min_max(
                    Pos2::new(x1, (y_center - block_h * 0.5).max(lane_top + pad_y)),
                    Pos2::new(x2, (y_center + block_h * 0.5).min(lane_top + lane_h - pad_y)),
                );

                painter.rect_filled(block_rect, 2.0, color);

                // Active note border glow
                if is_active {
                    painter.rect_stroke(block_rect, 2.0, Stroke::new(1.0, Color32::WHITE));
                }

                // Note label on active blocks with enough width
                if is_active && block_rect.width() > 22.0 {
                    let label = if ch == 9 {
                        drum_name(ev.midi_note)
                    } else {
                        midi_to_name(ev.midi_note)
                    };
                    painter.text(
                        Pos2::new(block_rect.left() + 3.0, block_rect.center().y),
                        egui::Align2::LEFT_CENTER,
                        label,
                        egui::FontId::monospace(8.0),
                        Color32::WHITE,
                    );
                }
            }
        }

        // ── Playhead (centered vertical line) ────────────────────
        painter.line_segment(
            [
                Pos2::new(playhead_x, rect.top() + header_h),
                Pos2::new(playhead_x, rect.bottom()),
            ],
            Stroke::new(2.0, Color32::WHITE),
        );

        // ── Status text ──────────────────────────────────────────
        let active_count = self.playing_events
            .iter()
            .filter(|e| e.tick <= tick && tick < e.tick + e.duration_ticks)
            .count();
        let measure = tick / (480 * 4) + 1;
        let beat_in_measure = (tick % (480 * 4)) / 480 + 1;
        painter.text(
            Pos2::new(rect.right() - 4.0, rect.top() + 2.0),
            egui::Align2::RIGHT_TOP,
            format!("{} notes  bar {}.{}", active_count, measure, beat_in_measure),
            egui::FontId::monospace(10.0),
            Color32::from_rgb(120, 122, 135),
        );

        ui.ctx().request_repaint();
    }
}

fn midi_to_name(midi: u8) -> String {
    let names = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];
    let octave = (midi as i8 / 12) - 1;
    let note = midi % 12;
    format!("{}{}", names[note as usize], octave)
}

fn drum_name(midi: u8) -> String {
    match midi {
        35 | 36 => "BD".into(),
        38 | 40 => "SD".into(),
        42 => "HH".into(),
        46 => "OH".into(),
        49 | 57 => "CR".into(),
        51 | 59 => "RD".into(),
        _ => format!("D{}", midi),
    }
}
