use egui::{Color32, Pos2, Rect, Stroke, Vec2};

/// Real-time audio visualizer: waveform display and spectrum analyzer.
pub struct Visualizer {
    /// Waveform sample buffer (ring buffer of recent audio samples).
    pub waveform: Vec<f32>,
    /// Spectrum magnitudes (FFT bins).
    pub spectrum: Vec<f32>,
    /// Toggle between waveform and spectrum view.
    pub mode: VisualizerMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VisualizerMode {
    Waveform,
    Spectrum,
    Both,
}

impl Visualizer {
    pub fn new() -> Self {
        Self {
            waveform: vec![0.0; 1024],
            spectrum: vec![0.0; 512],
            mode: VisualizerMode::Waveform,
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

    pub fn ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.mode, VisualizerMode::Waveform, "Waveform");
            ui.selectable_value(&mut self.mode, VisualizerMode::Spectrum, "Spectrum");
            ui.selectable_value(&mut self.mode, VisualizerMode::Both, "Both");
        });
        ui.separator();

        match self.mode {
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
}
