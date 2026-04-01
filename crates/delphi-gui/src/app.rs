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
                if ui.button("▶ Play (F5)").clicked() {
                    self.transport.play(&self.studio, &self.stop_flag, self.soundfont_mgr.active_path.as_ref());
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
            if i.key_pressed(egui::Key::F5) {
                self.transport.play(&self.studio, &self.stop_flag, self.soundfont_mgr.active_path.as_ref());
            }
            if i.key_pressed(egui::Key::Escape) {
                self.stop_flag.store(true, Ordering::SeqCst);
            }
            if i.modifiers.ctrl && i.key_pressed(egui::Key::S) {
                self.save_project();
            }
            if i.modifiers.ctrl && i.key_pressed(egui::Key::E) {
                self.export_dialog.open = true;
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
                    });
                    ui.separator();
                    match self.bottom_panel {
                        BottomPanel::Mixer => self.mixer.ui(ui, &mut self.studio),
                        BottomPanel::Visualizer => self.visualizer.ui(ui),
                        BottomPanel::Theory => self.theory.ui(ui),
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

        // Request repaint while playing (for visualizer/transport updates)
        if self.transport.is_playing() {
            ctx.request_repaint();
        }
    }
}
