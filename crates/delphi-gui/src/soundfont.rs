use std::path::{Path, PathBuf};

/// SoundFont manager: discover, select, and download SoundFont files.
///
/// Resolution order (matches the Python `soundfont.py`):
///   1. `DELPHI_SOUNDFONT` environment variable
///   2. `~/.delphi/soundfonts/` (user collection)
///   3. System dirs: `/usr/share/sounds/sf2/`, `/usr/share/soundfonts/`,
///      `/usr/local/share/soundfonts/`
pub struct SoundFontManager {
    pub active_path: Option<PathBuf>,
    pub available: Vec<SoundFontEntry>,
    pub status: String,
}

pub struct SoundFontEntry {
    pub name: String,
    pub path: PathBuf,
    pub size_mb: f64,
}

impl SoundFontManager {
    pub fn new() -> Self {
        let mut mgr = Self {
            active_path: None,
            available: Vec::new(),
            status: String::new(),
        };
        mgr.discover();
        mgr
    }

    /// Full discovery: env var, user dir, system dirs. Auto-selects the best default.
    fn discover(&mut self) {
        self.available.clear();

        // 1. DELPHI_SOUNDFONT env var (highest priority)
        if let Ok(env_path) = std::env::var("DELPHI_SOUNDFONT") {
            let p = PathBuf::from(&env_path);
            if p.is_file() {
                self.add_entry(&p);
                self.active_path = Some(p);
                self.status = format!("Loaded from $DELPHI_SOUNDFONT");
                // Continue scanning so the user sees everything available
            }
        }

        // 2. ~/.delphi/soundfonts/
        let user_dir = delphi_home().join("soundfonts");
        if user_dir.is_dir() {
            self.scan_dir(&user_dir);
        }

        // 3. System directories
        let system_dirs = [
            PathBuf::from("/usr/share/sounds/sf2"),
            PathBuf::from("/usr/share/soundfonts"),
            PathBuf::from("/usr/local/share/soundfonts"),
        ];
        for dir in &system_dirs {
            if dir.is_dir() {
                self.scan_dir(dir);
            }
        }

        // macOS common locations
        #[cfg(target_os = "macos")]
        {
            if let Ok(home) = std::env::var("HOME") {
                let mac_dir = PathBuf::from(home).join("Library/Audio/Sounds/Banks");
                if mac_dir.is_dir() {
                    self.scan_dir(&mac_dir);
                }
            }
        }

        // Auto-select: prefer GeneralUser in ~/.delphi, then first available
        if self.active_path.is_none() {
            // Look for GeneralUser first (the Delphi default)
            if let Some(entry) = self.available.iter().find(|e| {
                e.name.to_lowercase().contains("generaluser")
            }) {
                self.active_path = Some(entry.path.clone());
            } else if let Some(entry) = self.available.first() {
                self.active_path = Some(entry.path.clone());
            }
        }

        let count = self.available.len();
        if self.status.is_empty() {
            self.status = if count > 0 {
                format!("Found {} SoundFont(s)", count)
            } else {
                "No SoundFonts found — use Browse or place .sf2 files in ~/.delphi/soundfonts/"
                    .into()
            };
        }
    }

    fn add_entry(&mut self, path: &Path) {
        // Avoid duplicates
        if self.available.iter().any(|e| e.path == path) {
            return;
        }
        let size = std::fs::metadata(path)
            .map(|m| m.len() as f64 / (1024.0 * 1024.0))
            .unwrap_or(0.0);
        self.available.push(SoundFontEntry {
            name: path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into(),
            path: path.to_path_buf(),
            size_mb: size,
        });
    }

    fn scan_dir(&mut self, dir: &Path) {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file()
                    && path.extension().map_or(false, |e| {
                        e.eq_ignore_ascii_case("sf2") || e.eq_ignore_ascii_case("sf3")
                    })
                {
                    self.add_entry(&path);
                }
            }
        }
    }

    pub fn ui(&mut self, ui: &mut egui::Ui) {
        ui.heading("SoundFonts");
        ui.separator();

        // Active soundfont
        if let Some(ref path) = self.active_path {
            ui.label(
                egui::RichText::new(format!("Active: {}", path.display()))
                    .color(egui::Color32::from_rgb(152, 195, 121)),
            );
        } else {
            ui.label(
                egui::RichText::new("No SoundFont loaded")
                    .color(egui::Color32::from_rgb(224, 108, 117)),
            );
        }

        ui.separator();

        // Available soundfonts
        ui.label("Available:");
        let mut set_active: Option<PathBuf> = None;
        for entry in &self.available {
            ui.horizontal(|ui| {
                let is_active = self
                    .active_path
                    .as_ref()
                    .map_or(false, |p| p == &entry.path);
                let label = if is_active {
                    format!("● {} ({:.1} MB)", entry.name, entry.size_mb)
                } else {
                    format!("  {} ({:.1} MB)", entry.name, entry.size_mb)
                };
                if ui.selectable_label(is_active, label).clicked() {
                    set_active = Some(entry.path.clone());
                }
            });
        }
        if let Some(path) = set_active {
            self.active_path = Some(path);
        }

        ui.separator();

        // Browse for custom .sf2
        if ui.button("Browse for .sf2…").clicked() {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("SoundFont", &["sf2"])
                .pick_file()
            {
                let size = std::fs::metadata(&path)
                    .map(|m| m.len() as f64 / (1024.0 * 1024.0))
                    .unwrap_or(0.0);
                self.active_path = Some(path.clone());
                self.available.push(SoundFontEntry {
                    name: path
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .into(),
                    path,
                    size_mb: size,
                });
            }
        }

        // Refresh
        if ui.button("Refresh").clicked() {
            self.discover();
        }

        if !self.status.is_empty() {
            ui.separator();
            ui.label(
                egui::RichText::new(&self.status)
                    .small()
                    .color(egui::Color32::from_rgb(150, 150, 150)),
            );
        }
    }
}

/// Get the Delphi config home: ~/.delphi/
fn delphi_home() -> PathBuf {
    if let Some(home) = std::env::var_os("HOME") {
        PathBuf::from(home).join(".delphi")
    } else {
        PathBuf::from(".delphi")
    }
}
