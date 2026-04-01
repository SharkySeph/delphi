use egui::{Color32, Pos2, Rect, Sense, Stroke, Vec2};

use crate::studio::StudioState;

/// A note event displayed on the piano roll grid.
#[derive(Debug, Clone)]
pub struct RollNote {
    pub midi_note: u8,
    pub start_tick: u32,
    pub duration_ticks: u32,
    pub velocity: u8,
    pub track_idx: usize,
    pub selected: bool,
}

/// Visual piano roll for note editing.
pub struct PianoRoll {
    /// Horizontal zoom: pixels per tick.
    pub zoom_x: f32,
    /// Vertical zoom: pixels per semitone row.
    pub zoom_y: f32,
    /// Scroll offset in ticks.
    pub scroll_x: f32,
    /// Lowest visible MIDI note.
    pub scroll_y: f32,
    /// Snap grid: ticks per snap unit (480 = quarter note, 240 = eighth, etc.).
    pub snap_ticks: u32,
    /// Notes to display.
    pub notes: Vec<RollNote>,
    /// Currently selected note indices.
    pub selection: Vec<usize>,
}

impl PianoRoll {
    pub fn new() -> Self {
        Self {
            zoom_x: 0.2,
            zoom_y: 10.0,
            scroll_x: 0.0,
            scroll_y: 48.0,  // Start around C3
            snap_ticks: 240, // Eighth note
            notes: Vec::new(),
            selection: Vec::new(),
        }
    }

    /// Convert tick position to screen x coordinate.
    fn tick_to_x(&self, tick: u32, offset: f32) -> f32 {
        (tick as f32 - self.scroll_x) * self.zoom_x + offset
    }

    /// Convert screen x to tick position (snapped).
    fn x_to_tick(&self, x: f32, offset: f32) -> u32 {
        let raw = ((x - offset) / self.zoom_x + self.scroll_x) as i32;
        let raw = raw.max(0) as u32;
        // Snap
        (raw / self.snap_ticks) * self.snap_ticks
    }

    /// Convert MIDI note to screen y coordinate.
    fn note_to_y(&self, midi: u8, offset: f32, total_height: f32) -> f32 {
        // Higher notes at top
        total_height - (midi as f32 - self.scroll_y) * self.zoom_y + offset
    }

    pub fn ui(&mut self, ui: &mut egui::Ui, studio: &mut StudioState) {
        // Toolbar
        ui.horizontal(|ui| {
            ui.label("Snap:");
            egui::ComboBox::from_id_salt("snap_grid")
                .selected_text(snap_label(self.snap_ticks))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.snap_ticks, 480, "1/4");
                    ui.selectable_value(&mut self.snap_ticks, 240, "1/8");
                    ui.selectable_value(&mut self.snap_ticks, 120, "1/16");
                    ui.selectable_value(&mut self.snap_ticks, 60, "1/32");
                    ui.selectable_value(&mut self.snap_ticks, 960, "1/2");
                    ui.selectable_value(&mut self.snap_ticks, 1920, "1 bar");
                });

            ui.separator();
            ui.label("Zoom:");
            ui.add(egui::Slider::new(&mut self.zoom_x, 0.05..=1.0).logarithmic(true).text("H"));
            ui.add(egui::Slider::new(&mut self.zoom_y, 4.0..=24.0).text("V"));
        });

        ui.separator();

        // Piano roll canvas
        let available = ui.available_size();
        let (response, painter) = ui.allocate_painter(available, Sense::click_and_drag());
        let rect = response.rect;
        let piano_width = 40.0; // Width of the piano key labels on the left

        // Background
        painter.rect_filled(rect, 0.0, Color32::from_rgb(30, 30, 35));

        // Draw piano keys on the left
        let visible_notes = ((available.y / self.zoom_y) as u8).min(127);
        let lowest = self.scroll_y as u8;
        for i in 0..visible_notes {
            let midi = lowest + i;
            if midi > 127 {
                break;
            }
            let y = self.note_to_y(midi, rect.top(), available.y);
            let is_black = matches!(midi % 12, 1 | 3 | 6 | 8 | 10);

            // Key background
            let key_rect = Rect::from_min_size(
                Pos2::new(rect.left(), y - self.zoom_y * 0.5),
                Vec2::new(piano_width, self.zoom_y),
            );
            let key_color = if is_black {
                Color32::from_rgb(40, 40, 45)
            } else {
                Color32::from_rgb(55, 55, 60)
            };
            painter.rect_filled(key_rect, 0.0, key_color);

            // Grid line
            painter.line_segment(
                [
                    Pos2::new(rect.left() + piano_width, y),
                    Pos2::new(rect.right(), y),
                ],
                Stroke::new(0.5, Color32::from_rgb(50, 50, 55)),
            );

            // Note label (every C)
            if midi % 12 == 0 {
                let label = format!("C{}", (midi / 12) as i8 - 1);
                painter.text(
                    Pos2::new(rect.left() + 4.0, y),
                    egui::Align2::LEFT_CENTER,
                    label,
                    egui::FontId::monospace(9.0),
                    Color32::from_rgb(150, 150, 150),
                );
            }
        }

        // Draw beat grid lines
        let ticks_per_bar = 480 * 4; // Assuming 4/4
        let first_tick = self.scroll_x as u32;
        let last_tick = first_tick + (available.x / self.zoom_x) as u32;
        let mut tick = (first_tick / 480) * 480;
        while tick <= last_tick {
            let x = self.tick_to_x(tick, rect.left() + piano_width);
            let is_bar = tick % ticks_per_bar as u32 == 0;
            let color = if is_bar {
                Color32::from_rgb(80, 80, 85)
            } else {
                Color32::from_rgb(45, 45, 50)
            };
            painter.line_segment(
                [Pos2::new(x, rect.top()), Pos2::new(x, rect.bottom())],
                Stroke::new(if is_bar { 1.0 } else { 0.5 }, color),
            );
            tick += 480;
        }

        // Draw notes
        let track_colors = [
            Color32::from_rgb(86, 182, 194),
            Color32::from_rgb(229, 192, 123),
            Color32::from_rgb(152, 195, 121),
            Color32::from_rgb(224, 108, 117),
            Color32::from_rgb(198, 120, 221),
            Color32::from_rgb(209, 154, 102),
            Color32::from_rgb(97, 175, 239),
            Color32::from_rgb(190, 80, 70),
        ];

        for (i, note) in self.notes.iter().enumerate() {
            let x = self.tick_to_x(note.start_tick, rect.left() + piano_width);
            let w = note.duration_ticks as f32 * self.zoom_x;
            let y = self.note_to_y(note.midi_note, rect.top(), available.y);

            let base_color = track_colors[note.track_idx % track_colors.len()];
            let color = if note.selected {
                Color32::WHITE
            } else {
                let alpha = 100 + (note.velocity as u16 * 155 / 127) as u8;
                Color32::from_rgba_premultiplied(
                    base_color.r(),
                    base_color.g(),
                    base_color.b(),
                    alpha,
                )
            };

            let note_rect = Rect::from_min_size(
                Pos2::new(x, y - self.zoom_y * 0.4),
                Vec2::new(w.max(2.0), self.zoom_y * 0.8),
            );
            painter.rect_filled(note_rect, 2.0, color);

            if note.selected {
                painter.rect_stroke(note_rect, 2.0, Stroke::new(1.5, Color32::WHITE));
            }
        }

        // Handle scroll
        if response.hovered() {
            let scroll = ui.input(|i| i.smooth_scroll_delta);
            self.scroll_x = (self.scroll_x - scroll.x / self.zoom_x).max(0.0);
            self.scroll_y = (self.scroll_y - scroll.y / self.zoom_y).clamp(0.0, 115.0);
        }
    }
}

fn snap_label(ticks: u32) -> &'static str {
    match ticks {
        1920 => "1 bar",
        960 => "1/2",
        480 => "1/4",
        240 => "1/8",
        120 => "1/16",
        60 => "1/32",
        _ => "?",
    }
}
