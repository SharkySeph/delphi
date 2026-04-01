use eframe::CreationContext;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
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
                        self.studio.load(&path);
                        self.project_path = Some(path);
                    }
                    ui.close_menu();
                }
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
                    self.transport.play(&self.studio, &self.stop_flag, self.soundfont_mgr.active_path.as_ref());
                    ui.close_menu();
                }
                if ui.button("▶ Play Cell (F6)").clicked() {
                    let idx = self.editor.active_cell;
                    self.transport.play_cell(&self.studio, idx, &self.stop_flag, self.soundfont_mgr.active_path.as_ref());
                    ui.close_menu();
                }
                if ui.button("⏹ Stop (Esc)").clicked() {
                    self.stop_flag.store(true, Ordering::SeqCst);
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
}

impl eframe::App for DelphiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Keyboard shortcuts
        ctx.input(|i| {
            // F5: Play
            if i.key_pressed(egui::Key::F5) {
                self.transport.play(&self.studio, &self.stop_flag, self.soundfont_mgr.active_path.as_ref());
            }
            // Escape: Stop
            if i.key_pressed(egui::Key::Escape) {
                self.stop_flag.store(true, Ordering::SeqCst);
            }
            // F6: Run current cell
            if i.key_pressed(egui::Key::F6) {
                let idx = self.editor.active_cell;
                self.transport.play_cell(&self.studio, idx, &self.stop_flag, self.soundfont_mgr.active_path.as_ref());
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
            self.transport.ui(ui, &self.studio, &self.stop_flag, self.soundfont_mgr.active_path.as_ref());
        });

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
            self.transport.play_cell(&self.studio, idx, &self.stop_flag, self.soundfont_mgr.active_path.as_ref());
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
