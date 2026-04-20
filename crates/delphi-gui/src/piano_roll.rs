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

/// Active editing tool.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PianoTool {
    Select,
    Draw,
    Erase,
}

/// Piano roll with selection, draw, and erase tools.
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
    /// Active editing tool.
    pub tool: PianoTool,
    /// Default velocity for drawn notes.
    pub draw_velocity: u8,
    /// Dirty flag: hash of cell sources last synced.
    sync_hash: u64,
    /// When true, manual edits exist that haven't been written back.
    dirty_edits: bool,
}

impl PianoRoll {
    pub fn new() -> Self {
        Self {
            zoom_x: 0.2,
            zoom_y: 10.0,
            scroll_x: 0.0,
            scroll_y: 48.0,
            snap_ticks: 240,
            notes: Vec::new(),
            selection: Vec::new(),
            tool: PianoTool::Select,
            draw_velocity: 80,
            sync_hash: 0,
            dirty_edits: false,
        }
    }

    fn tick_to_x(&self, tick: u32, offset: f32) -> f32 {
        (tick as f32 - self.scroll_x) * self.zoom_x + offset
    }

    fn x_to_tick(&self, x: f32, offset: f32) -> u32 {
        let raw = ((x - offset) / self.zoom_x + self.scroll_x) as i32;
        let raw = raw.max(0) as u32;
        (raw / self.snap_ticks) * self.snap_ticks
    }

    fn y_to_note(&self, y: f32, offset: f32, total_height: f32) -> u8 {
        let midi = (total_height - y + offset) / self.zoom_y + self.scroll_y;
        (midi.round() as u8).clamp(0, 127)
    }

    fn note_to_y(&self, midi: u8, offset: f32, total_height: f32) -> f32 {
        total_height - (midi as f32 - self.scroll_y) * self.zoom_y + offset
    }

    fn note_at(&self, pos: Pos2, rect: Rect, piano_width: f32) -> Option<usize> {
        let available = rect.size();
        for (i, note) in self.notes.iter().enumerate().rev() {
            let x = self.tick_to_x(note.start_tick, rect.left() + piano_width);
            let w = note.duration_ticks as f32 * self.zoom_x;
            let y = self.note_to_y(note.midi_note, rect.top(), available.y);
            let note_rect = Rect::from_min_size(
                Pos2::new(x, y - self.zoom_y * 0.4),
                Vec2::new(w.max(2.0), self.zoom_y * 0.8),
            );
            if note_rect.contains(pos) {
                return Some(i);
            }
        }
        None
    }

    pub fn ui(&mut self, ui: &mut egui::Ui, studio: &mut StudioState) {
        if !self.dirty_edits {
            self.sync_from_studio_if_dirty(studio);
        }

        // Toolbar
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("Piano Roll")
                    .color(Color32::from_rgb(150, 150, 160)),
            );
            ui.separator();

            ui.selectable_value(&mut self.tool, PianoTool::Select, "⬚ Select");
            ui.selectable_value(&mut self.tool, PianoTool::Draw, "✏ Draw");
            ui.selectable_value(&mut self.tool, PianoTool::Erase, "✕ Erase");

            ui.separator();
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
            ui.label("Vel:");
            ui.add(egui::DragValue::new(&mut self.draw_velocity).range(1..=127).speed(1));

            ui.separator();
            ui.label("Zoom:");
            ui.add(egui::Slider::new(&mut self.zoom_x, 0.05..=1.0).logarithmic(true).text("H"));
            ui.add(egui::Slider::new(&mut self.zoom_y, 4.0..=24.0).text("V"));

            let sel_count = self.selection.len();
            if sel_count > 0 {
                ui.separator();
                ui.label(
                    egui::RichText::new(format!("{} selected", sel_count))
                        .small()
                        .color(Color32::from_rgb(86, 182, 194)),
                );
            }

            if self.dirty_edits {
                ui.separator();
                if ui.button("💾 Apply to cells").on_hover_text("Write piano roll edits back to notation cells").clicked() {
                    self.write_back_to_studio(studio);
                    self.dirty_edits = false;
                    self.sync_hash = 0;
                }
            }
        });

        ui.separator();

        // Keyboard actions
        let delete_pressed = ui.input(|i| i.key_pressed(egui::Key::Delete) || i.key_pressed(egui::Key::Backspace));
        let select_all = ui.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::A));

        if delete_pressed && !self.selection.is_empty() {
            let mut to_remove = self.selection.clone();
            to_remove.sort_unstable();
            to_remove.dedup();
            for &idx in to_remove.iter().rev() {
                if idx < self.notes.len() {
                    self.notes.remove(idx);
                }
            }
            self.selection.clear();
            self.dirty_edits = true;
        }

        if select_all {
            self.selection = (0..self.notes.len()).collect();
            for note in &mut self.notes {
                note.selected = true;
            }
        }

        // Canvas
        let available = ui.available_size();
        let (response, painter) = ui.allocate_painter(available, Sense::click_and_drag());
        let rect = response.rect;
        let piano_width = 40.0;

        painter.rect_filled(rect, 0.0, Color32::from_rgb(30, 30, 35));

        // Piano keys
        let visible_notes = ((available.y / self.zoom_y) as u8).min(127);
        let lowest = self.scroll_y as u8;
        for i in 0..visible_notes {
            let midi = lowest + i;
            if midi > 127 { break; }
            let y = self.note_to_y(midi, rect.top(), available.y);
            let is_black = matches!(midi % 12, 1 | 3 | 6 | 8 | 10);

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

            painter.line_segment(
                [
                    Pos2::new(rect.left() + piano_width, y),
                    Pos2::new(rect.right(), y),
                ],
                Stroke::new(0.5, Color32::from_rgb(50, 50, 55)),
            );

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

        // Beat grid
        let ticks_per_bar: u32 = 480 * 4;
        let first_tick = self.scroll_x as u32;
        let last_tick = first_tick + (available.x / self.zoom_x) as u32;
        let mut tick = (first_tick / 480) * 480;
        while tick <= last_tick {
            let x = self.tick_to_x(tick, rect.left() + piano_width);
            let is_bar = tick % ticks_per_bar == 0;
            let color = if is_bar {
                Color32::from_rgb(80, 80, 85)
            } else {
                Color32::from_rgb(45, 45, 50)
            };
            painter.line_segment(
                [Pos2::new(x, rect.top()), Pos2::new(x, rect.bottom())],
                Stroke::new(if is_bar { 1.0 } else { 0.5 }, color),
            );
            if is_bar {
                let bar_num = tick / ticks_per_bar + 1;
                painter.text(
                    Pos2::new(x + 3.0, rect.top() + 2.0),
                    egui::Align2::LEFT_TOP,
                    format!("{}", bar_num),
                    egui::FontId::monospace(9.0),
                    Color32::from_rgb(100, 100, 110),
                );
            }
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

        for note in self.notes.iter() {
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

        // Mouse interaction
        if response.clicked() {
            if let Some(pos) = response.interact_pointer_pos() {
                if pos.x > rect.left() + piano_width {
                    match self.tool {
                        PianoTool::Select => {
                            let shift = ui.input(|i| i.modifiers.shift);
                            if let Some(idx) = self.note_at(pos, rect, piano_width) {
                                if shift {
                                    self.notes[idx].selected = !self.notes[idx].selected;
                                    if self.notes[idx].selected {
                                        self.selection.push(idx);
                                    } else {
                                        self.selection.retain(|&i| i != idx);
                                    }
                                } else {
                                    for note in &mut self.notes { note.selected = false; }
                                    self.selection.clear();
                                    self.notes[idx].selected = true;
                                    self.selection.push(idx);
                                }
                            } else if !shift {
                                for note in &mut self.notes { note.selected = false; }
                                self.selection.clear();
                            }
                        }
                        PianoTool::Draw => {
                            let tick = self.x_to_tick(pos.x, rect.left() + piano_width);
                            let midi = self.y_to_note(pos.y, rect.top(), available.y);
                            self.notes.push(RollNote {
                                midi_note: midi,
                                start_tick: tick,
                                duration_ticks: self.snap_ticks,
                                velocity: self.draw_velocity,
                                track_idx: 0,
                                selected: false,
                            });
                            self.dirty_edits = true;
                        }
                        PianoTool::Erase => {
                            if let Some(idx) = self.note_at(pos, rect, piano_width) {
                                self.notes.remove(idx);
                                self.selection.retain(|&i| i != idx);
                                for s in &mut self.selection {
                                    if *s > idx { *s -= 1; }
                                }
                                self.dirty_edits = true;
                            }
                        }
                    }
                }
            }
        }

        // Scroll
        if response.hovered() {
            let scroll = ui.input(|i| i.smooth_scroll_delta);
            self.scroll_x = (self.scroll_x - scroll.x / self.zoom_x).max(0.0);
            self.scroll_y = (self.scroll_y - scroll.y / self.zoom_y).clamp(0.0, 115.0);
        }

        // Hover tooltip
        if response.hovered() {
            if let Some(pos) = ui.input(|i| i.pointer.hover_pos()) {
                if pos.x > rect.left() + piano_width {
                    let midi = self.y_to_note(pos.y, rect.top(), available.y);
                    let tick = self.x_to_tick(pos.x, rect.left() + piano_width);
                    let bar = tick / ticks_per_bar + 1;
                    let beat = (tick % ticks_per_bar) / 480 + 1;
                    let name = midi_note_name(midi);
                    response.clone().on_hover_text(format!("{} — Bar {} Beat {}", name, bar, beat));
                }
            }
        }
    }

    fn sync_from_studio_if_dirty(&mut self, studio: &StudioState) {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        for cell in &studio.cells {
            cell.source.hash(&mut hasher);
            cell.instrument.hash(&mut hasher);
            cell.channel.hash(&mut hasher);
            cell.velocity.hash(&mut hasher);
        }
        let new_hash = hasher.finish();
        if new_hash == self.sync_hash {
            return;
        }
        self.sync_hash = new_hash;
        self.sync_from_studio(studio);
    }

    fn sync_from_studio(&mut self, studio: &StudioState) {
        use crate::studio::{gm_program_from_name, parse_notation_to_events};

        let key_name = &studio.settings.key_name;
        let key_opt: Option<&str> = if key_name.is_empty() { None } else { Some(key_name) };

        self.notes.clear();
        self.selection.clear();
        for (cell_idx, cell) in studio.cells.iter().enumerate() {
            if cell.cell_type == "markdown" || cell.source.trim().is_empty() {
                continue;
            }
            let program = gm_program_from_name(&cell.instrument);
            let events = parse_notation_to_events(&cell.source, cell.channel, program, cell.velocity, key_opt);
            for ev in events {
                self.notes.push(RollNote {
                    midi_note: ev.midi_note,
                    start_tick: ev.tick,
                    duration_ticks: ev.duration_ticks,
                    velocity: ev.velocity,
                    track_idx: cell_idx,
                    selected: false,
                });
            }
        }
    }

    fn write_back_to_studio(&self, studio: &mut StudioState) {
        let mut by_track: std::collections::BTreeMap<usize, Vec<&RollNote>> = std::collections::BTreeMap::new();
        for note in &self.notes {
            by_track.entry(note.track_idx).or_default().push(note);
        }

        for (&cell_idx, notes) in &by_track {
            if cell_idx >= studio.cells.len() { continue; }
            let cell = &mut studio.cells[cell_idx];
            if cell.cell_type == "markdown" || cell.cell_type == "code" { continue; }

            let mut pragmas: Vec<String> = Vec::new();
            for line in cell.source.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with("# @") || trimmed.starts_with("// @") {
                    pragmas.push(line.to_string());
                }
            }

            let mut sorted: Vec<&&RollNote> = notes.iter().collect();
            sorted.sort_by_key(|n| (n.start_tick, n.midi_note));

            let mut output = String::new();
            for pragma in &pragmas {
                output.push_str(pragma);
                output.push('\n');
            }
            if !pragmas.is_empty() { output.push('\n'); }

            let mut col = 0;
            for note in sorted {
                let name = midi_note_name(note.midi_note);
                let dur = ticks_to_duration(note.duration_ticks);
                output.push_str(&format!("{}:{}", name, dur));
                if note.velocity != cell.velocity && note.velocity != 80 {
                    let dyn_mark = velocity_to_dynamic(note.velocity);
                    output.push_str(&format!("!{}", dyn_mark));
                }
                output.push(' ');
                col += 1;
                if col % 8 == 0 { output.push('\n'); }
            }

            cell.source = output.trim_end().to_string();
            cell.source.push('\n');
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

fn midi_note_name(midi: u8) -> String {
    const NAMES: &[&str] = &["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];
    let octave = (midi / 12) as i8 - 1;
    let name = NAMES[(midi % 12) as usize];
    format!("{}{}", name, octave)
}

fn ticks_to_duration(ticks: u32) -> &'static str {
    match ticks {
        3840 => "dw",
        1920 => "w",
        1440 => "h.",
        960 => "h",
        720 => "q.",
        480 => "q",
        360 => "8.",
        320 => "qt",
        240 => "8",
        160 => "8t",
        120 => "16",
        60 => "32",
        30 => "64",
        _ => "q",
    }
}

fn velocity_to_dynamic(vel: u8) -> &'static str {
    match vel {
        0..=24 => "ppp",
        25..=41 => "pp",
        42..=56 => "p",
        57..=72 => "mp",
        73..=88 => "mf",
        89..=104 => "f",
        105..=119 => "ff",
        120..=127 => "fff",
        _ => "mf",
    }
}
