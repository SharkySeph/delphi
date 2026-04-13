use egui::text::LayoutJob;
use egui::{Color32, FontId, TextFormat};

use crate::studio::{StudioState, GM_INSTRUMENT_NAMES};

/// Syntax token types for .delphi notation coloring.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TokenKind {
    Note,        // C4, D#5, Bb3
    Chord,       // Cmaj7, Am, G7
    Duration,    // :q, :8, :h, :w
    Barline,     // |
    Rest,        // .
    Bracket,     // [ ] < > ( )
    Drum,        // kick, snare, hihat
    Operator,    // ~, *, !, @, ?
    Pragma,      // @instrument, @channel
    Comment,     // // ...
    String,      // "..."
    Number,      // 120, 0.5
    Keyword,     // tempo, key, time_sig, instrument, etc.
    Plain,       // everything else
}

/// Tokenize a line of .delphi notation for syntax highlighting.
fn tokenize_line(line: &str) -> Vec<(TokenKind, usize, usize)> {
    let mut tokens = Vec::new();
    let chars: Vec<char> = line.chars().collect();
    let len = chars.len();
    let mut i = 0;

    // Comment check
    if line.trim_start().starts_with("//") {
        tokens.push((TokenKind::Comment, 0, len));
        return tokens;
    }

    // Pragma line
    if line.trim_start().starts_with('@') {
        tokens.push((TokenKind::Pragma, 0, len));
        return tokens;
    }

    while i < len {
        let ch = chars[i];

        // Whitespace — skip
        if ch.is_whitespace() {
            i += 1;
            continue;
        }

        // Barline
        if ch == '|' {
            tokens.push((TokenKind::Barline, i, i + 1));
            i += 1;
            continue;
        }

        // Brackets
        if matches!(ch, '[' | ']' | '<' | '>' | '(' | ')') {
            tokens.push((TokenKind::Bracket, i, i + 1));
            i += 1;
            continue;
        }

        // Operators
        if matches!(ch, '~' | '*' | '!' | '@' | '?') {
            tokens.push((TokenKind::Operator, i, i + 1));
            i += 1;
            continue;
        }

        // Rest
        if ch == '.' && (i + 1 >= len || chars[i + 1].is_whitespace() || chars[i + 1] == '|') {
            tokens.push((TokenKind::Rest, i, i + 1));
            i += 1;
            continue;
        }

        // Duration suffix: :q :8 :h :w :16 :32 etc.
        if ch == ':' {
            let start = i;
            i += 1;
            while i < len && (chars[i].is_alphanumeric() || chars[i] == '.') {
                i += 1;
            }
            tokens.push((TokenKind::Duration, start, i));
            continue;
        }

        // String literal
        if ch == '"' {
            let start = i;
            i += 1;
            while i < len && chars[i] != '"' {
                i += 1;
            }
            if i < len {
                i += 1; // closing quote
            }
            tokens.push((TokenKind::String, start, i));
            continue;
        }

        // Word token (note, chord, drum, keyword, or plain)
        if ch.is_alphanumeric() || ch == '#' || ch == '-' {
            let start = i;
            while i < len
                && !chars[i].is_whitespace()
                && !matches!(chars[i], '|' | '[' | ']' | '<' | '>' | '(' | ')' | '~' | ':')
            {
                i += 1;
            }
            let word: String = chars[start..i].iter().collect();
            let kind = classify_word(&word);
            tokens.push((kind, start, i));
            continue;
        }

        // Comma (polyphony separator) — plain
        tokens.push((TokenKind::Plain, i, i + 1));
        i += 1;
    }

    tokens
}

fn classify_word(word: &str) -> TokenKind {
    // Keywords
    let kw = word.to_lowercase();
    if matches!(
        kw.as_str(),
        "tempo" | "key" | "time_sig" | "instrument" | "swing" | "humanize" | "section" | "track"
    ) {
        return TokenKind::Keyword;
    }

    // Drum names
    if matches!(
        kw.as_str(),
        "kick" | "bd" | "snare" | "sd" | "hihat" | "hh" | "openhat" | "oh" | "ride" | "rd"
            | "crash" | "cr" | "clap" | "cp" | "tom1" | "tom2" | "tom3" | "rimshot" | "rim"
            | "cowbell" | "cb" | "tambourine" | "tamb" | "shaker" | "clave" | "woodblock" | "wb"
            | "triangle" | "tri" | "guiro"
    ) {
        return TokenKind::Drum;
    }

    // Note: letter + optional accidental + octave number
    // e.g. C4, D#5, Bb3, F##2
    let bytes = word.as_bytes();
    if !bytes.is_empty() && bytes[0].is_ascii_alphabetic() {
        let first = (bytes[0] as char).to_ascii_uppercase();
        if matches!(first, 'A'..='G') {
            // Check if remaining looks like accidental + octave
            let rest = &word[1..];
            let rest_trimmed = rest
                .trim_start_matches('#')
                .trim_start_matches('b');
            if rest_trimmed.is_empty()
                || rest_trimmed.chars().next().map_or(false, |c| c.is_ascii_digit() || c == '-')
            {
                // Could be a note (C4, Bb, F#5) or a chord (Cmaj7, Am7)
                if rest_trimmed.is_empty()
                    || rest_trimmed.chars().all(|c| c.is_ascii_digit() || c == '-')
                {
                    return TokenKind::Note;
                }
            }
            // Chord: letter + accidental + quality
            return TokenKind::Chord;
        }
    }

    // Number
    if word.chars().all(|c| c.is_ascii_digit() || c == '.') {
        return TokenKind::Number;
    }

    TokenKind::Plain
}

/// The main code editor state.
pub struct EditorState {
    /// The active cell index in the studio (which cell is being edited).
    pub active_cell: usize,
    /// Cell index to play (set by ▶ button, consumed by app.rs).
    pub cell_to_run: Option<usize>,
    /// Cell index that was just run (for output feedback, consumed by app.rs).
    pub last_run_cell: Option<usize>,
    /// Autocomplete state
    pub completion_open: bool,
    pub completion_items: Vec<&'static str>,
    pub completion_filter: String,
    pub completion_selected: usize,
    /// Live diagnostics per cell (cell index → warnings).
    pub cell_diagnostics: Vec<Vec<String>>,
    /// Tracks which cell sources have been seen (for change detection).
    cell_hashes: Vec<u64>,
}

impl EditorState {
    pub fn new() -> Self {
        Self {
            active_cell: 0,
            cell_to_run: None,
            last_run_cell: None,
            completion_open: false,
            completion_items: Vec::new(),
            completion_filter: String::new(),
            completion_selected: 0,
            cell_diagnostics: Vec::new(),
            cell_hashes: Vec::new(),
        }
    }

    pub fn ui(&mut self, ui: &mut egui::Ui, studio: &mut StudioState, active_highlights: &[Vec<(usize, usize)>]) {
        let cell_count = studio.cells.len();
        if cell_count == 0 {
            ui.centered_and_justified(|ui| {
                if ui.button("+ Add Cell").clicked() {
                    studio.add_cell();
                }
            });
            return;
        }

        // Cell toolbar
        ui.horizontal(|ui| {
            if ui.button("+ Code").clicked() {
                studio.add_cell();
            }
            if ui.button("+ Notation").clicked() {
                studio.add_notation_cell();
            }
            if ui.button("+ Markdown").clicked() {
                studio.add_markdown_cell();
            }
            ui.separator();
            ui.label(format!("{} cells", cell_count));
        });
        ui.separator();

        // Scrollable cell list
        egui::ScrollArea::vertical().show(ui, |ui| {
            let mut to_delete: Option<usize> = None;
            let mut to_run: Option<usize> = None;
            let mut to_move_up: Option<usize> = None;
            let mut to_move_down: Option<usize> = None;
            let mut needs_channel_reassign = false;

            let cell_count_inner = studio.cells.len();
            let editor_key_name = studio.settings.key_name.clone();
            let editor_key_opt: Option<&str> = if editor_key_name.is_empty() { None } else { Some(&editor_key_name) };

            for (idx, cell) in studio.cells.iter_mut().enumerate() {
                let is_active = idx == self.active_cell;
                let frame = if is_active {
                    egui::Frame::group(ui.style())
                        .stroke(egui::Stroke::new(2.0, Color32::from_rgb(100, 149, 237)))
                } else {
                    egui::Frame::group(ui.style())
                };

                frame.show(ui, |ui| {
                    // Cell header
                    ui.horizontal(|ui| {
                        // Collapse toggle
                        let collapse_label = if cell.collapsed { "▶" } else { "▼" };
                        if ui.small_button(collapse_label).clicked() {
                            cell.collapsed = !cell.collapsed;
                        }

                        let type_label = match cell.cell_type.as_str() {
                            "notation" => "♪",
                            "markdown" => "📄",
                            _ => "▸",
                        };
                        ui.label(format!("[{}] {}", idx + 1, type_label));

                        if !cell.label.is_empty() {
                            ui.label(
                                egui::RichText::new(&cell.label)
                                    .color(Color32::from_rgb(200, 200, 100)),
                            );
                        }

                        // Instrument selector
                        if cell.cell_type != "markdown" {
                            let prev = cell.instrument.clone();
                            egui::ComboBox::from_id_salt(format!("cell_inst_{}", idx))
                                .selected_text(format!("🎵 {}", cell.instrument))
                                .width(120.0)
                                .show_ui(ui, |ui| {
                                    for &name in GM_INSTRUMENT_NAMES {
                                        ui.selectable_value(&mut cell.instrument, name.to_string(), name);
                                    }
                                });
                            if cell.instrument != prev {
                                // Force diagnostics re-check and clear stale output
                                if idx < self.cell_hashes.len() {
                                    self.cell_hashes[idx] = 0;
                                }
                                cell.output.clear();
                                needs_channel_reassign = true;
                            }
                        }

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.small_button("🗑").clicked() {
                                to_delete = Some(idx);
                            }
                            if cell.cell_type != "markdown" && ui.small_button("▶").clicked() {
                                to_run = Some(idx);
                            }
                            // Reorder buttons
                            if idx + 1 < cell_count_inner && ui.small_button("↓").clicked() {
                                to_move_down = Some(idx);
                            }
                            if idx > 0 && ui.small_button("↑").clicked() {
                                to_move_up = Some(idx);
                            }
                        });
                    });

                    if cell.collapsed {
                        // Show a brief preview when collapsed
                        let preview: String = cell.source.lines().next().unwrap_or("").chars().take(60).collect();
                        if !preview.is_empty() {
                            ui.label(
                                egui::RichText::new(preview)
                                    .small()
                                    .color(Color32::from_rgb(120, 120, 140)),
                            );
                        }
                    } else {
                    // Code editor with syntax highlighting (with playback token glow)
                    let cell_hl: &[(usize, usize)] = if idx < active_highlights.len() {
                        &active_highlights[idx]
                    } else {
                        &[]
                    };
                    let mut layouter = |ui: &egui::Ui, text: &str, wrap_width: f32| {
                        let layout_job = highlight_notation(ui, text, wrap_width, cell_hl);
                        ui.fonts(|f| f.layout_job(layout_job))
                    };
                    let editor_response = ui.add(
                        egui::TextEdit::multiline(&mut cell.source)
                            .code_editor()
                            .desired_width(f32::INFINITY)
                            .desired_rows(4)
                            .layouter(&mut layouter),
                    );
                    if editor_response.clicked() {
                        self.active_cell = idx;
                    }

                    // --- Live error checking ---
                    // Detect source changes via simple hash and re-run diagnostics
                    {
                        use std::collections::hash_map::DefaultHasher;
                        use std::hash::{Hash, Hasher};
                        while self.cell_diagnostics.len() <= idx {
                            self.cell_diagnostics.push(Vec::new());
                            self.cell_hashes.push(0);
                        }
                        let mut hasher = DefaultHasher::new();
                        cell.source.hash(&mut hasher);
                        let new_hash = hasher.finish();
                        if new_hash != self.cell_hashes[idx] {
                            self.cell_hashes[idx] = new_hash;
                            if cell.cell_type == "notation" && !cell.source.trim().is_empty() {
                                let (_events, mut warnings) = crate::studio::parse_notation_with_diagnostics(
                                    &cell.source,
                                    cell.channel,
                                    crate::studio::gm_program_from_name(&cell.instrument),
                                    cell.velocity,
                                    editor_key_opt,
                                );
                                // Warn if cell instrument is unrecognized
                                if !cell.instrument.is_empty()
                                    && crate::studio::gm_program_from_name_checked(&cell.instrument).is_none()
                                {
                                    warnings.insert(0, format!(
                                        "unknown instrument '{}' — defaulting to piano",
                                        cell.instrument,
                                    ));
                                }
                                self.cell_diagnostics[idx] = warnings;
                            } else {
                                self.cell_diagnostics[idx].clear();
                            }
                        }
                    }

                    // Show inline diagnostics (warnings/errors) below editor
                    if idx < self.cell_diagnostics.len() && !self.cell_diagnostics[idx].is_empty() {
                        for warning in &self.cell_diagnostics[idx] {
                            ui.horizontal(|ui| {
                                ui.label(
                                    egui::RichText::new("⚠")
                                        .color(Color32::from_rgb(229, 192, 123)),
                                );
                                ui.label(
                                    egui::RichText::new(warning)
                                        .small()
                                        .color(Color32::from_rgb(229, 192, 123)),
                                );
                            });
                        }
                    }

                    // --- Autocomplete ---
                    if is_active && editor_response.has_focus() {
                        // Ctrl+Space to trigger, or auto-trigger on typing
                        let ctrl_space = ui.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::Space));
                        let source_changed = editor_response.changed();

                        if ctrl_space || (source_changed && self.completion_open) {
                            // Extract the current word being typed (last token before cursor)
                            let src = &cell.source;
                            let prefix = extract_current_word(src);
                            self.completion_filter = prefix.to_lowercase();
                            self.completion_items = get_completions(&self.completion_filter);
                            self.completion_open = !self.completion_items.is_empty();
                            self.completion_selected = 0;
                        }

                        // Auto-open on typing if we have items
                        if source_changed && !self.completion_open {
                            let prefix = extract_current_word(&cell.source);
                            if prefix.len() >= 2 {
                                self.completion_filter = prefix.to_lowercase();
                                self.completion_items = get_completions(&self.completion_filter);
                                if !self.completion_items.is_empty() {
                                    self.completion_open = true;
                                    self.completion_selected = 0;
                                }
                            }
                        }

                        // Keyboard nav for completion list
                        if self.completion_open {
                            let nav = ui.input(|i| {
                                if i.key_pressed(egui::Key::Escape) {
                                    return CompletionAction::Close;
                                }
                                if i.key_pressed(egui::Key::ArrowDown) {
                                    return CompletionAction::Down;
                                }
                                if i.key_pressed(egui::Key::ArrowUp) {
                                    return CompletionAction::Up;
                                }
                                if i.key_pressed(egui::Key::Tab) || i.key_pressed(egui::Key::Enter) {
                                    return CompletionAction::Accept;
                                }
                                CompletionAction::None
                            });

                            match nav {
                                CompletionAction::Close => {
                                    self.completion_open = false;
                                }
                                CompletionAction::Down => {
                                    if !self.completion_items.is_empty() {
                                        self.completion_selected = (self.completion_selected + 1) % self.completion_items.len();
                                    }
                                }
                                CompletionAction::Up => {
                                    if !self.completion_items.is_empty() {
                                        self.completion_selected = if self.completion_selected == 0 {
                                            self.completion_items.len() - 1
                                        } else {
                                            self.completion_selected - 1
                                        };
                                    }
                                }
                                CompletionAction::Accept => {
                                    if let Some(&item) = self.completion_items.get(self.completion_selected) {
                                        apply_completion(&mut cell.source, &self.completion_filter, item);
                                        self.completion_open = false;
                                    }
                                }
                                CompletionAction::None => {}
                            }
                        }

                        // Render completion list below the editor
                        if self.completion_open && !self.completion_items.is_empty() {
                            egui::Frame::popup(ui.style())
                                .show(ui, |ui| {
                                    ui.set_min_width(200.0);
                                    for (ci, item) in self.completion_items.iter().enumerate() {
                                        let selected = ci == self.completion_selected;
                                        let label = if selected {
                                            egui::RichText::new(*item)
                                                .strong()
                                                .color(Color32::from_rgb(86, 182, 194))
                                        } else {
                                            egui::RichText::new(*item)
                                                .color(Color32::from_rgb(171, 178, 191))
                                        };
                                        if ui.selectable_label(selected, label).clicked() {
                                            apply_completion(&mut cell.source, &self.completion_filter, item);
                                            self.completion_open = false;
                                        }
                                    }
                                });
                        }
                    } else if !editor_response.has_focus() {
                        // Close completion when editor loses focus
                        self.completion_open = false;
                    }

                    } // end !collapsed else block

                    // Output area
                    if !cell.output.is_empty() {
                        ui.separator();
                        ui.label(
                            egui::RichText::new(&cell.output)
                                .color(Color32::from_rgb(150, 150, 150))
                                .monospace(),
                        );
                    }
                });

                ui.add_space(4.0);
            }

            if let Some(idx) = to_delete {
                studio.cells.remove(idx);
                if self.active_cell >= studio.cells.len() && !studio.cells.is_empty() {
                    self.active_cell = studio.cells.len() - 1;
                }
                needs_channel_reassign = true;
            }

            if let Some(idx) = to_run {
                self.cell_to_run = Some(idx);
            }

            if let Some(idx) = to_move_up {
                if studio.move_cell_up(idx) {
                    self.active_cell = idx - 1;
                }
            }

            if let Some(idx) = to_move_down {
                if studio.move_cell_down(idx) {
                    self.active_cell = idx + 1;
                }
            }

            if needs_channel_reassign {
                studio.auto_assign_channels();
            }
        });
    }
}

/// Produce a syntax-highlighted LayoutJob for .delphi notation.
/// When `highlights` is non-empty, tokens whose byte range overlaps an active
/// highlight get a colored background glow (Strudel-style playback indicator).
fn highlight_notation(ui: &egui::Ui, text: &str, wrap_width: f32, highlights: &[(usize, usize)]) -> LayoutJob {
    let mut job = LayoutJob::default();
    job.wrap.max_width = wrap_width;

    let font = FontId::monospace(14.0);
    let style = ui.style();
    let default_color = style.visuals.text_color();

    let highlight_bg = Color32::from_rgba_premultiplied(86, 182, 194, 55); // Teal glow

    let mut line_byte_start: usize = 0;

    for (line_idx, line) in text.split('\n').enumerate() {
        if line_idx > 0 {
            job.append("\n", 0.0, TextFormat::simple(font.clone(), default_color));
        }

        let tokens = tokenize_line(line);
        let line_chars: Vec<char> = line.chars().collect();
        let mut pos = 0;

        for (kind, start, end) in &tokens {
            // Plain text before this token
            if *start > pos {
                let gap: String = line_chars[pos..*start].iter().collect();
                job.append(&gap, 0.0, TextFormat::simple(font.clone(), default_color));
            }

            let span_text: String = line_chars[*start..*end].iter().collect();
            let color = token_color(*kind);

            // Check if this token overlaps any active highlight
            let byte_s = line_byte_start + *start;
            let byte_e = line_byte_start + *end;
            let is_lit = !highlights.is_empty()
                && highlights
                    .iter()
                    .any(|(hs, he)| byte_s < *he && byte_e > *hs);

            if is_lit {
                job.append(
                    &span_text,
                    0.0,
                    TextFormat {
                        font_id: font.clone(),
                        color: Color32::WHITE,
                        background: highlight_bg,
                        ..Default::default()
                    },
                );
            } else {
                job.append(&span_text, 0.0, TextFormat::simple(font.clone(), color));
            }
            pos = *end;
        }

        // Remaining text after last token
        if pos < line_chars.len() {
            let rest: String = line_chars[pos..].iter().collect();
            job.append(&rest, 0.0, TextFormat::simple(font.clone(), default_color));
        }

        line_byte_start += line.len() + 1; // +1 for '\n'
    }

    job
}

fn token_color(kind: TokenKind) -> Color32 {
    match kind {
        TokenKind::Note => Color32::from_rgb(86, 182, 194),        // Teal
        TokenKind::Chord => Color32::from_rgb(229, 192, 123),      // Gold
        TokenKind::Duration => Color32::from_rgb(152, 195, 121),   // Green
        TokenKind::Barline => Color32::from_rgb(120, 120, 120),    // Gray
        TokenKind::Rest => Color32::from_rgb(100, 100, 100),       // Dark gray
        TokenKind::Bracket => Color32::from_rgb(198, 120, 221),    // Purple
        TokenKind::Drum => Color32::from_rgb(224, 108, 117),       // Red
        TokenKind::Operator => Color32::from_rgb(198, 120, 221),   // Purple
        TokenKind::Pragma => Color32::from_rgb(209, 154, 102),     // Orange
        TokenKind::Comment => Color32::from_rgb(92, 99, 112),      // Dim gray
        TokenKind::String => Color32::from_rgb(152, 195, 121),     // Green
        TokenKind::Number => Color32::from_rgb(209, 154, 102),     // Orange
        TokenKind::Keyword => Color32::from_rgb(198, 120, 221),    // Purple
        TokenKind::Plain => Color32::from_rgb(171, 178, 191),      // Light gray
    }
}

/// Return auto-complete suggestions matching the given prefix.
fn get_completions(prefix: &str) -> Vec<&'static str> {
    static ALL_COMPLETIONS: &[&str] = &[
        // Notes
        "C4", "C#4", "Db4", "D4", "D#4", "Eb4", "E4", "F4", "F#4", "Gb4",
        "G4", "G#4", "Ab4", "A4", "A#4", "Bb4", "B4",
        "C3", "D3", "E3", "F3", "G3", "A3", "B3",
        "C5", "D5", "E5", "F5", "G5", "A5", "B5",
        // Chords
        "Cmaj7", "Dm7", "Em7", "Fmaj7", "G7", "Am7", "Bm7b5",
        "C", "Cm", "Cdim", "Caug", "C7", "Cmaj7", "Cm7",
        "D", "Dm", "D7", "Dm7", "Dmaj7",
        "E", "Em", "E7", "Em7",
        "F", "Fm", "F7", "Fmaj7",
        "G", "Gm", "G7", "Gmaj7",
        "A", "Am", "A7", "Am7", "Amaj7",
        "B", "Bm", "B7", "Bm7", "Bdim",
        // Durations
        ":w", ":h", ":q", ":8", ":16", ":32",
        ":w.", ":h.", ":q.", ":8.",
        ":8t", ":qt", ":ht",
        ":dw",
        // Dynamics
        "!ppp", "!pp", "!p", "!mp", "!mf", "!f", "!ff", "!fff", "!sfz",
        // Articulations
        ".stac", ".ten", ".acc", ".marc", ".ferm", ".ghost", ".leg", ".pizz", ".mute",
        // Ornaments
        ".tr", ".mord", ".lmord", ".turn", ".grace", ".appoggiatura", ".trem", ".gliss", ".arp", ".roll",
        // Drums
        "kick", "snare", "hihat", "openhat", "ride", "crash",
        "clap", "rimshot", "cowbell", "tambourine", "shaker",
        "tom1", "tom2", "tom3", "woodblock", "triangle",
        // Rests
        "r", "rest",
        // Pragmas
        "# @instrument ", "# @track ", "# @channel ", "# @velocity ",
        // Structural
        "breath", "caesura",
        // Expression
        "cresc(", "dim(",
    ];

    if prefix.is_empty() {
        return Vec::new();
    }

    ALL_COMPLETIONS
        .iter()
        .filter(|item| item.to_lowercase().starts_with(prefix))
        .copied()
        .take(12)
        .collect()
}

/// Actions for keyboard navigation in the completion popup.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CompletionAction {
    None,
    Close,
    Up,
    Down,
    Accept,
}

/// Extract the current word being typed (the last partial token in the source).
/// This finds the last whitespace/delimiter boundary and returns everything after it.
fn extract_current_word(src: &str) -> &str {
    let delimiters = |c: char| c.is_whitespace() || matches!(c, '|' | '[' | ']' | '(' | ')' | '{' | '}');
    match src.rfind(delimiters) {
        Some(pos) => &src[pos + 1..],
        None => src,
    }
}

/// Apply a completion: replace the last partial word in source with the chosen item.
fn apply_completion(source: &mut String, filter: &str, item: &str) {
    if filter.is_empty() {
        source.push_str(item);
        source.push(' ');
        return;
    }
    // Find the last occurrence of the filter text (case-insensitive) and replace it
    let lower_src = source.to_lowercase();
    let lower_filter = filter.to_lowercase();
    if let Some(pos) = lower_src.rfind(&lower_filter) {
        let end = pos + filter.len();
        source.replace_range(pos..end, item);
        // Add trailing space if not already present
        if !source.ends_with(' ') {
            source.push(' ');
        }
    }
}
