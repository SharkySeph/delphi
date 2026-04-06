use eframe::CreationContext;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use crate::editor::EditorState;
use crate::export::ExportDialog;
use crate::mixer::MixerPanel;
use crate::piano_roll::PianoRoll;
use crate::player::TransportState;
use crate::scripting::ScriptEngine;
use crate::soundfont::SoundFontManager;
use crate::studio::StudioState;
use crate::theme::DelphiTheme;
use crate::theory::TheoryPanel;
use crate::visualizer::Visualizer;

/// Which panel is visible in the center area.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CenterPanel {
    Editor,
    PianoRoll,
}

/// Which panel is visible in the bottom area.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BottomPanel {
    Mixer,
    Visualizer,
    Theory,
    Help,
}

/// Which panel is visible in the right sidebar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SidePanel {
    Tracks,
    SoundFonts,
    Export,
    Script,
}

/// Top-level application state.
pub struct DelphiApp {
    pub theme: DelphiTheme,

    // Core state
    pub studio: StudioState,
    pub editor: EditorState,
    pub transport: TransportState,
    pub mixer: MixerPanel,
    pub piano_roll: PianoRoll,
    pub visualizer: Visualizer,
    pub theory: TheoryPanel,
    pub soundfont_mgr: SoundFontManager,
    pub export_dialog: ExportDialog,
    pub script_engine: ScriptEngine,

    // Layout toggles
    pub center_panel: CenterPanel,
    pub bottom_panel: BottomPanel,
    pub side_panel: SidePanel,
    pub show_bottom: bool,
    pub show_side: bool,

    // Engine state
    pub stop_flag: Arc<AtomicBool>,
    pub project_path: Option<PathBuf>,
}

impl DelphiApp {
    pub fn new(cc: &CreationContext) -> Self {
        let theme = DelphiTheme::default();
        theme.apply(&cc.egui_ctx);

        Self {
            theme,
            studio: StudioState::new(),
            editor: EditorState::new(),
            transport: TransportState::new(),
            mixer: MixerPanel::new(),
            piano_roll: PianoRoll::new(),
            visualizer: Visualizer::new(),
            theory: TheoryPanel::new(),
            soundfont_mgr: SoundFontManager::new(),
            export_dialog: ExportDialog::new(),
            script_engine: ScriptEngine::new(),
            center_panel: CenterPanel::Editor,
            bottom_panel: BottomPanel::Mixer,
            side_panel: SidePanel::Tracks,
            show_bottom: true,
            show_side: true,
            stop_flag: Arc::new(AtomicBool::new(false)),
            project_path: None,
        }
    }

    fn menu_bar(&mut self, ui: &mut egui::Ui) {
        egui::menu::bar(ui, |ui| {
            ui.menu_button("File", |ui| {
                if ui.button("New Project").clicked() {
                    self.studio = StudioState::new();
                    self.project_path = None;
                    ui.close_menu();
                }
                if ui.button("Open…").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("Delphi Studio", &["dstudio"])
                        .add_filter("Delphi", &["delphi"])
                        .pick_file()
                    {
                        match self.studio.load(&path) {
                            Ok(()) => self.project_path = Some(path),
                            Err(e) => {
                                // Show error inline — the status will be visible
                                eprintln!("Failed to open project: {}", e);
                            }
                        }
                    }
                    ui.close_menu();
                }
                ui.menu_button("Open Example", |ui| {
                    if ui.button("Hello World").clicked() {
                        self.load_example("Hello World", EXAMPLE_HELLO);
                        ui.close_menu();
                    }
                    if ui.button("Twinkle Twinkle").clicked() {
                        self.load_example("Twinkle Twinkle", EXAMPLE_TWINKLE);
                        ui.close_menu();
                    }
                    if ui.button("12-Bar Blues").clicked() {
                        self.load_example("12-Bar Blues", EXAMPLE_BLUES);
                        ui.close_menu();
                    }
                    if ui.button("Canon in D").clicked() {
                        self.load_example("Canon in D", EXAMPLE_CANON);
                        ui.close_menu();
                    }
                    if ui.button("Studio Showcase").clicked() {
                        self.load_example_dstudio();
                        ui.close_menu();
                    }
                });
                if ui.button("Save").clicked() {
                    self.save_project();
                    ui.close_menu();
                }
                if ui.button("Save As…").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("Delphi Studio", &["dstudio"])
                        .save_file()
                    {
                        self.project_path = Some(path.clone());
                        self.studio.save(&path);
                    }
                    ui.close_menu();
                }
                ui.separator();
                if ui.button("Export…").clicked() {
                    self.export_dialog.sf_path = self.soundfont_mgr.active_path.clone();
                    self.export_dialog.master_gain = self.mixer.master_gain;
                    self.export_dialog.open = true;
                    ui.close_menu();
                }
            });

            ui.menu_button("View", |ui| {
                if ui.button("Editor").clicked() {
                    self.center_panel = CenterPanel::Editor;
                    ui.close_menu();
                }
                if ui.button("Piano Roll").clicked() {
                    self.center_panel = CenterPanel::PianoRoll;
                    ui.close_menu();
                }
                ui.separator();
                ui.checkbox(&mut self.show_bottom, "Bottom Panel");
                ui.checkbox(&mut self.show_side, "Side Panel");
            });

            ui.menu_button("Transport", |ui| {
                if ui.button("▶ Play All (F5)").clicked() {
                    self.transport.play(&self.studio, &self.stop_flag, self.soundfont_mgr.active_path.as_ref(), self.mixer.master_gain);
                    ui.close_menu();
                }
                if ui.button("▶ Play Cell (F6)").clicked() {
                    let idx = self.editor.active_cell;
                    self.transport.play_cell(&self.studio, idx, &self.stop_flag, self.soundfont_mgr.active_path.as_ref(), self.mixer.master_gain);
                    ui.close_menu();
                }
                if ui.button("⏹ Stop (Esc)").clicked() {
                    self.transport.stop(&self.stop_flag);
                    ui.close_menu();
                }
            });

            ui.menu_button("Tools", |ui| {
                if ui.button("Theory Explorer").clicked() {
                    self.show_bottom = true;
                    self.bottom_panel = BottomPanel::Theory;
                    ui.close_menu();
                }
                if ui.button("SoundFont Manager").clicked() {
                    self.show_side = true;
                    self.side_panel = SidePanel::SoundFonts;
                    ui.close_menu();
                }
                if ui.button("Script Console").clicked() {
                    self.show_side = true;
                    self.side_panel = SidePanel::Script;
                    ui.close_menu();
                }
            });

            ui.menu_button("Help", |ui| {
                if ui.button("Quick Reference (Ctrl+H)").clicked() {
                    self.show_bottom = true;
                    self.bottom_panel = BottomPanel::Help;
                    ui.close_menu();
                }
                ui.separator();
                if ui.button("About Delphi Studio").clicked() {
                    // Simple inline about
                    ui.close_menu();
                }
            });
        });
    }

    fn save_project(&mut self) {
        if let Some(ref path) = self.project_path {
            self.studio.save(path);
        } else if let Some(path) = rfd::FileDialog::new()
            .add_filter("Delphi Studio", &["dstudio"])
            .save_file()
        {
            self.project_path = Some(path.clone());
            self.studio.save(&path);
        }
    }

    fn load_example(&mut self, title: &str, notation: &str) {
        self.studio = StudioState::new();
        self.studio.settings.title = title.to_string();
        self.studio.cells.clear();
        let mut cell = crate::studio::Cell::new_notation();
        cell.source = notation.to_string();
        self.studio.cells.push(cell);
        self.project_path = None;
    }

    fn load_example_dstudio(&mut self) {
        self.studio = StudioState::new();
        let json = EXAMPLE_SHOWCASE;
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(json) {
            // Use load_from_python_format logic via a temp file approach
            // Or parse inline — simpler to just parse the JSON directly
            self.studio.settings.title = v["title"].as_str().unwrap_or("Showcase").to_string();
            if let Some(s) = v.get("settings") {
                self.studio.settings.bpm = s["tempo"].as_f64().unwrap_or(120.0);
                if let Some(k) = s["key"].as_str() {
                    self.studio.settings.key_name = k.to_string();
                }
            }
            self.studio.cells.clear();
            if let Some(cells) = v["cells"].as_array() {
                for c in cells {
                    let mut cell = match c["type"].as_str().unwrap_or("notation") {
                        "markdown" => crate::studio::Cell::new_markdown(),
                        "code" => crate::studio::Cell::new_code(),
                        _ => crate::studio::Cell::new_notation(),
                    };
                    cell.source = c["source"].as_str().unwrap_or("").to_string();
                    if let Some(meta) = c.get("meta") {
                        if let Some(label) = meta["label"].as_str() {
                            cell.label = label.to_string();
                        }
                        if let Some(prog) = meta["program"].as_str() {
                            cell.instrument = prog.to_string();
                        }
                        if let Some(ch) = meta["channel"].as_u64() {
                            cell.channel = ch as u8;
                        }
                    }
                    // Also parse pragmas from source
                    for line in cell.source.lines() {
                        let trimmed: &str = line.trim();
                        if let Some(rest) = trimmed.strip_prefix("# @instrument ") {
                            cell.instrument = rest.to_string();
                        } else if let Some(rest) = trimmed.strip_prefix("# @channel ") {
                            if let Ok(ch) = rest.parse::<u8>() {
                                cell.channel = ch;
                            }
                        } else if let Some(rest) = trimmed.strip_prefix("# @velocity ") {
                            if let Ok(v) = rest.parse::<u8>() {
                                cell.velocity = v;
                            }
                        } else if let Some(rest) = trimmed.strip_prefix("# @track ") {
                            cell.label = rest.to_string();
                        }
                    }
                    self.studio.cells.push(cell);
                }
            }
            // Build tracks from cells
            for cell in &self.studio.cells {
                if cell.cell_type == "markdown" || cell.source.trim().is_empty() {
                    continue;
                }
                let name = if cell.label.is_empty() {
                    cell.instrument.clone()
                } else {
                    cell.label.clone()
                };
                if name.is_empty() {
                    continue;
                }
                if !self.studio.tracks.iter().any(|t| t.name == name) {
                    self.studio.tracks.push(crate::studio::TrackState {
                        name,
                        instrument: cell.instrument.clone(),
                        program: crate::studio::gm_program_from_name(&cell.instrument),
                        channel: cell.channel,
                        gain: 1.0,
                        pan: 0.5,
                        muted: false,
                        solo: false,
                    });
                }
            }
        }
        self.project_path = None;
    }
}

/// Built-in example: Hello World
const EXAMPLE_HELLO: &str = "\
// Hello Delphi — your first song!
// @instrument piano
// @track Melody

C4:q E4:q G4:q C5:q
| C:q | Am:q | F:q | G:q |
| C:q | Am:q | F:q | G:q |
C4:h E4:h G4:w
";

/// Built-in example: Twinkle Twinkle
const EXAMPLE_TWINKLE: &str = "\
// Twinkle, Twinkle, Little Star
// @instrument piano
// @track Melody
// @velocity 90

C4:q C4:q G4:q G4:q  A4:q A4:q G4:h
F4:q F4:q E4:q E4:q  D4:q D4:q C4:h

G4:q G4:q F4:q F4:q  E4:q E4:q D4:h
G4:q G4:q F4:q F4:q  E4:q E4:q D4:h

C4:q C4:q G4:q G4:q  A4:q A4:q G4:h
F4:q F4:q E4:q E4:q  D4:q D4:q C4:h
";

/// Built-in example: 12-Bar Blues in A
const EXAMPLE_BLUES: &str = "\
// 12-Bar Blues in A
// @instrument piano
// @track Blues

| A7:w | A7:w | A7:w | A7:w |
| D7:w | D7:w | A7:w | A7:w |
| E7:w | D7:w | A7:w | E7:w |
";

/// Built-in example: Canon in D (simplified notation)
const EXAMPLE_CANON: &str = "\
// Pachelbel's Canon in D (Melody)
// @instrument violin
// @track Violin 1
// @velocity 85

F#5:q E5:q  D5:q  C#5:q
B4:q  A4:q  B4:q  C#5:q
D5:q  C#5:q B4:q  A4:q
G4:q  F#4:q G4:q  E4:q

D4:8  F#4:8 A4:8  G4:8  F#4:8 D4:8  F#4:8 E4:8
D4:8  B3:8  D4:8  A4:8  G4:8  B4:8  A4:8  G4:8
F#4:8 D4:8  E4:8  C#5:8 D5:8  F#5:8 A5:8  A4:8
B4:8  G4:8  A4:8  F#4:8 D4:8  D5:8  D5:8  C#5:8
";

/// Built-in example: Studio Showcase (.dstudio JSON)
const EXAMPLE_SHOWCASE: &str = include_str!("../../../examples/showcase.dstudio");

impl eframe::App for DelphiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Keyboard shortcuts
        ctx.input(|i| {
            // F5: Play
            if i.key_pressed(egui::Key::F5) {
                self.transport.play(&self.studio, &self.stop_flag, self.soundfont_mgr.active_path.as_ref(), self.mixer.master_gain);
            }
            // Escape: Stop
            if i.key_pressed(egui::Key::Escape) {
                self.transport.stop(&self.stop_flag);
            }
            // F6: Run current cell
            if i.key_pressed(egui::Key::F6) {
                let idx = self.editor.active_cell;
                self.transport.play_cell(&self.studio, idx, &self.stop_flag, self.soundfont_mgr.active_path.as_ref(), self.mixer.master_gain);
            }
            // F7: Add cell
            if i.key_pressed(egui::Key::F7) {
                self.studio.add_cell();
                self.editor.active_cell = self.studio.cells.len().saturating_sub(1);
            }
            // F8: Delete current cell
            if i.key_pressed(egui::Key::F8) {
                let idx = self.editor.active_cell;
                if idx < self.studio.cells.len() {
                    self.studio.cells.remove(idx);
                    if self.editor.active_cell >= self.studio.cells.len() && !self.studio.cells.is_empty() {
                        self.editor.active_cell = self.studio.cells.len() - 1;
                    }
                }
            }
            // Ctrl+S: Save
            if i.modifiers.ctrl && i.key_pressed(egui::Key::S) {
                self.save_project();
            }
            // Ctrl+E: Export
            if i.modifiers.ctrl && i.key_pressed(egui::Key::E) {
                self.export_dialog.sf_path = self.soundfont_mgr.active_path.clone();
                self.export_dialog.master_gain = self.mixer.master_gain;
                self.export_dialog.open = true;
            }
            // Ctrl+Up: Navigate to previous cell
            if i.modifiers.ctrl && !i.modifiers.shift && i.key_pressed(egui::Key::ArrowUp) {
                if self.editor.active_cell > 0 {
                    self.editor.active_cell -= 1;
                }
            }
            // Ctrl+Down: Navigate to next cell
            if i.modifiers.ctrl && !i.modifiers.shift && i.key_pressed(egui::Key::ArrowDown) {
                if self.editor.active_cell + 1 < self.studio.cells.len() {
                    self.editor.active_cell += 1;
                }
            }
            // Ctrl+Shift+Up: Move cell up
            if i.modifiers.ctrl && i.modifiers.shift && i.key_pressed(egui::Key::ArrowUp) {
                let idx = self.editor.active_cell;
                if self.studio.move_cell_up(idx) {
                    self.editor.active_cell = idx - 1;
                }
            }
            // Ctrl+Shift+Down: Move cell down
            if i.modifiers.ctrl && i.modifiers.shift && i.key_pressed(egui::Key::ArrowDown) {
                let idx = self.editor.active_cell;
                if self.studio.move_cell_down(idx) {
                    self.editor.active_cell = idx + 1;
                }
            }
            // Ctrl+H: Toggle help panel
            if i.modifiers.ctrl && i.key_pressed(egui::Key::H) {
                self.show_bottom = true;
                self.bottom_panel = BottomPanel::Help;
            }
        });

        // Top menu bar
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            self.menu_bar(ui);
        });

        // Transport bar (below menu)
        egui::TopBottomPanel::top("transport_bar").show(ctx, |ui| {
            self.transport.ui(ui, &self.studio, &self.stop_flag, self.soundfont_mgr.active_path.as_ref(), self.mixer.master_gain);
        });

        // Sync transport BPM override into project settings so exports use it
        if let Some(bpm) = self.transport.bpm_override {
            self.studio.settings.bpm = bpm;
        }

        // Bottom panel (mixer / visualizer / theory)
        if self.show_bottom {
            egui::TopBottomPanel::bottom("bottom_panel")
                .resizable(true)
                .default_height(200.0)
                .show(ctx, |ui| {
                    // Tab bar
                    ui.horizontal(|ui| {
                        ui.selectable_value(&mut self.bottom_panel, BottomPanel::Mixer, "Mixer");
                        ui.selectable_value(
                            &mut self.bottom_panel,
                            BottomPanel::Visualizer,
                            "Visualizer",
                        );
                        ui.selectable_value(&mut self.bottom_panel, BottomPanel::Theory, "Theory");
                        ui.selectable_value(&mut self.bottom_panel, BottomPanel::Help, "Help");
                    });
                    ui.separator();
                    match self.bottom_panel {
                        BottomPanel::Mixer => self.mixer.ui(ui, &mut self.studio),
                        BottomPanel::Visualizer => self.visualizer.ui(ui),
                        BottomPanel::Theory => self.theory.ui(ui),
                        BottomPanel::Help => help_panel_ui(ui),
                    }
                });
        }

        // Right sidebar (tracks / soundfonts / export / script)
        if self.show_side {
            egui::SidePanel::right("side_panel")
                .resizable(true)
                .default_width(260.0)
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.selectable_value(&mut self.side_panel, SidePanel::Tracks, "Tracks");
                        ui.selectable_value(
                            &mut self.side_panel,
                            SidePanel::SoundFonts,
                            "SF",
                        );
                        ui.selectable_value(&mut self.side_panel, SidePanel::Export, "Export");
                        ui.selectable_value(&mut self.side_panel, SidePanel::Script, "Script");
                    });
                    ui.separator();
                    match self.side_panel {
                        SidePanel::Tracks => self.studio.tracks_ui(ui),
                        SidePanel::SoundFonts => self.soundfont_mgr.ui(ui),
                        SidePanel::Export => self.export_dialog.panel_ui(ui, &self.studio),
                        SidePanel::Script => self.script_engine.ui(ui, &mut self.studio),
                    }
                });
        }

        // Center: editor or piano roll
        egui::CentralPanel::default().show(ctx, |ui| {
            // Tab bar for center view
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.center_panel, CenterPanel::Editor, "📝 Editor");
                ui.selectable_value(
                    &mut self.center_panel,
                    CenterPanel::PianoRoll,
                    "🎹 Piano Roll",
                );
            });
            ui.separator();
            match self.center_panel {
                CenterPanel::Editor => self.editor.ui(ui, &mut self.studio),
                CenterPanel::PianoRoll => self.piano_roll.ui(ui, &mut self.studio),
            }
        });

        // Export dialog (modal)
        self.export_dialog.modal_ui(ctx, &self.studio);

        // Handle cell run request from editor's ▶ button
        if let Some(idx) = self.editor.cell_to_run.take() {
            self.transport.play_cell(&self.studio, idx, &self.stop_flag, self.soundfont_mgr.active_path.as_ref(), self.mixer.master_gain);
            self.editor.last_run_cell = Some(idx);
        }

        // Update visualizer with playback state
        {
            let events = self.studio.collect_events_mixed(None, self.mixer.master_gain);
            let bpm = self.transport.bpm_override.unwrap_or(self.studio.settings.bpm);
            let playing = self.transport.is_playing();
            let elapsed = self.transport.elapsed_secs();
            self.visualizer.update_playback(&events, elapsed, bpm, playing);
        }

        // Write cell output after running
        if let Some(idx) = self.editor.last_run_cell.take() {
            if idx < self.studio.cells.len() {
                let cell = &self.studio.cells[idx];
                let (events, warnings) = crate::studio::parse_notation_with_diagnostics(
                    &cell.source,
                    cell.channel,
                    crate::studio::gm_program_from_name(&cell.instrument),
                    cell.velocity,
                );
                let bars = if events.is_empty() {
                    0.0
                } else {
                    let max_tick = events.iter().map(|e| e.tick + e.duration_ticks).max().unwrap_or(0);
                    max_tick as f64 / (480.0 * 4.0)
                };
                let mut output = format!(
                    "♪ {} notes, {:.1} bars [{}]",
                    events.len(),
                    bars,
                    if cell.instrument.is_empty() { "piano" } else { &cell.instrument },
                );
                if !warnings.is_empty() {
                    output.push_str(&format!("\n⚠ {}", warnings.join("; ")));
                }
                self.studio.cells[idx].output = output;
            }
        }

        // Request repaint while playing (for visualizer/transport updates)
        if self.transport.is_playing() {
            ctx.request_repaint();
        }
    }
}

/// Built-in help / quick reference panel.
fn help_panel_ui(ui: &mut egui::Ui) {
    egui::ScrollArea::vertical().show(ui, |ui| {
        ui.heading("Delphi Studio — Quick Reference");
        ui.separator();

        ui.label(egui::RichText::new("Keyboard Shortcuts").strong());
        egui::Grid::new("help_shortcuts").striped(true).show(ui, |ui| {
            let row = |ui: &mut egui::Ui, key: &str, desc: &str| {
                ui.label(egui::RichText::new(key).monospace().color(egui::Color32::from_rgb(86, 182, 194)));
                ui.label(desc);
                ui.end_row();
            };
            row(ui, "F5", "Play all cells");
            row(ui, "Escape", "Stop playback");
            row(ui, "F6", "Play current cell");
            row(ui, "F7", "Add new cell");
            row(ui, "F8", "Delete current cell");
            row(ui, "Ctrl+S", "Save project");
            row(ui, "Ctrl+E", "Open export dialog");
            row(ui, "Ctrl+H", "Toggle this help panel");
            row(ui, "Ctrl+Up/Down", "Navigate cells");
            row(ui, "Ctrl+Shift+Up/Down", "Reorder cells");
        });

        ui.separator();
        ui.label(egui::RichText::new("Notation Syntax").strong());
        egui::Grid::new("help_notation").striped(true).show(ui, |ui| {
            let row = |ui: &mut egui::Ui, syntax: &str, desc: &str| {
                ui.label(egui::RichText::new(syntax).monospace().color(egui::Color32::from_rgb(229, 192, 123)));
                ui.label(desc);
                ui.end_row();
            };
            row(ui, "C4 D#5 Bb3", "Notes (pitch + octave)");
            row(ui, "Cmaj7 Am G7", "Chord symbols");
            row(ui, ":q :h :w :8 :16", "Duration (quarter, half, whole, 8th, 16th)");
            row(ui, ":q.  :h.", "Dotted durations");
            row(ui, ". ~ r", "Rest");
            row(ui, "|", "Bar line (visual only)");
            row(ui, "C4,E4,G4", "Polyphony (simultaneous notes)");
            row(ui, "!p !mf !ff", "Dynamics (pp, p, mp, mf, f, ff, fff)");
            row(ui, "kick snare hihat", "Drum names (channel 10)");
            row(ui, "kick(3,8)", "Euclidean rhythm (hits, steps)");
            row(ui, "@instrument piano", "Pragma (cell metadata)");
            row(ui, "// comment", "Comment line");
        });

        ui.separator();
        ui.label(egui::RichText::new("Cell Types").strong());
        ui.label("• Code / Notation — parsed and played");
        ui.label("• Markdown — documentation, not played");

        ui.separator();
        ui.label(egui::RichText::new("Export Formats").strong());
        ui.label("• MIDI (.mid) — Standard MIDI File, Format 1");
        ui.label("• WAV (.wav) — Audio render via SoundFont (requires SF2 loaded)");
        ui.label("• MusicXML (.xml) — Coming soon");

        ui.separator();
        ui.label(egui::RichText::new("GM Instruments").strong());
        ui.label("Set via track sidebar or @instrument pragma:");
        ui.label(
            egui::RichText::new(
                "piano, electric piano, organ, violin, cello, strings, trumpet, flute, \
                 bass, acoustic guitar, electric guitar, sax, choir, drums …"
            )
            .small()
            .color(egui::Color32::from_rgb(150, 150, 150)),
        );
    });
}
