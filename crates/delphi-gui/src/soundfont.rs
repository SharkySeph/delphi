use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use crossbeam_channel::{Receiver, TryRecvError};
use delphi_engine::{SoundFontCompatibilityReport, TrackCompatibilityIssueKind, audit_soundfont_compatibility};
use reqwest::blocking::Client;
use reqwest::header::CONTENT_TYPE;

use crate::studio::StudioState;

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
    /// Project-requested SoundFont that is currently missing on disk.
    missing_project_path: Option<PathBuf>,
    /// Whether the "Get SoundFonts" catalog section is expanded.
    show_catalog: bool,
    /// Clipboard-copied URL (shown briefly as feedback).
    copy_feedback: Option<(String, Instant)>,
    /// Background install worker result channel.
    install_rx: Option<Receiver<Result<InstallResult, String>>>,
    /// Human-readable name of the catalog item currently being installed.
    installing_name: Option<String>,
    compatibility_report: Option<SoundFontCompatibilityReport>,
    compatibility_key: Option<u64>,
}

pub struct SoundFontEntry {
    pub name: String,
    pub path: PathBuf,
    pub size_mb: f64,
}

/// A curated, freely redistributable SoundFont with license provenance.
struct CatalogEntry {
    /// Short display name.
    name: &'static str,
    /// SPDX or informal license identifier.
    license: &'static str,
    /// One-line description of the font.
    description: &'static str,
    /// Catalog / project page for the font.
    url: &'static str,
    /// Direct download URL, when the upstream source provides one.
    download_url: Option<&'static str>,
    /// Recommended install path relative to `~/.delphi/soundfonts/`.
    filename: &'static str,
    /// Notes shown in the UI for install limitations or behavior.
    install_note: &'static str,
}

struct InstallResult {
    installed_path: PathBuf,
    message: String,
}

struct InstallSpec {
    name: String,
    download_url: String,
    filename: String,
}

/// Curated catalog of freely available SoundFonts.
/// Only fonts with verified, clearly documented licensing are included.
/// Delphi does not redistribute these files.
static CATALOG: &[CatalogEntry] = &[
    CatalogEntry {
        name: "GeneralUser GS",
        license: "CC BY 4.0",
        description: "Excellent GM/GS bank by S. Christian Collins — the Delphi default.",
        url: "https://schristiancollins.com/generaluser.php",
        download_url: Some("https://drive.google.com/uc?export=download&id=12ZzM70Nxnr4vqyUF0bbRKE_HXQgLRNid"),
        filename: "GeneralUser_GS.sf2",
        install_note: "One-click install downloads the current release archive and extracts the main SoundFont into ~/.delphi/soundfonts/.",
    },
    CatalogEntry {
        name: "FluidR3 GM",
        license: "MIT / LGPL 2.1",
        description: "Reference GM bank included with many Linux distributions.",
        url: "https://member.keymusician.com/Member/FluidR3_GM/index.html",
        download_url: None,
        filename: "FluidR3_GM.sf2",
        install_note: "Manual for now: the upstream page does not expose a verified stable direct file URL.",
    },
    CatalogEntry {
        name: "MuseScore General",
        license: "MIT",
        description: "High-quality GM bank used by MuseScore, with acoustic and electric piano.",
        url: "https://ftp.osuosl.org/pub/musescore/soundfont/MuseScore_General/",
        download_url: Some("https://ftp.osuosl.org/pub/musescore/soundfont/MuseScore_General/MuseScore_General.sf3"),
        filename: "MuseScore_General.sf3",
        install_note: "One-click install saves the upstream .sf3 directly into ~/.delphi/soundfonts/.",
    },
    CatalogEntry {
        name: "Arachno SoundFont",
        license: "Custom free (non-commercial)",
        description: "Balanced GM bank with a warm vintage character.",
        url: "https://www.arachnosoft.com/main/soundfont.php",
        download_url: None,
        filename: "Arachno SoundFont - Version 1.0.sf2",
        install_note: "Manual for now: the source site currently redirects instead of exposing a stable direct download.",
    },
];

impl SoundFontManager {
    pub fn new() -> Self {
        let mut mgr = Self {
            active_path: None,
            available: Vec::new(),
            status: String::new(),
            missing_project_path: None,
            show_catalog: false,
            copy_feedback: None,
            install_rx: None,
            installing_name: None,
            compatibility_report: None,
            compatibility_key: None,
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
                self.status = "Loaded from $DELPHI_SOUNDFONT".into();
                // Continue scanning so the user sees everything available
            } else {
                self.status = format!("Warning: $DELPHI_SOUNDFONT path not found: {}", env_path);
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
                "No SoundFonts found — see Get SoundFonts below, or use Browse".into()
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

    fn poll_install(&mut self) {
        let result = match self.install_rx.as_ref() {
            Some(rx) => match rx.try_recv() {
                Ok(result) => Some(result),
                Err(TryRecvError::Empty) => None,
                Err(TryRecvError::Disconnected) => {
                    Some(Err("installer worker disconnected unexpectedly".into()))
                }
            },
            None => None,
        };

        if let Some(result) = result {
            self.install_rx = None;
            self.installing_name = None;
            match result {
                Ok(result) => {
                    self.discover();
                    self.activate_path(result.installed_path);
                    self.status = result.message;
                }
                Err(err) => {
                    self.status = format!("Install failed: {err}");
                }
            }
        }
    }

    fn start_install(&mut self, entry: &'static CatalogEntry) {
        let Some(download_url) = entry.download_url else {
            self.status = format!("{} requires manual download", entry.name);
            return;
        };

        let installed_path = user_soundfont_dir().join(entry.filename);
        if installed_path.is_file() {
            self.activate_path(installed_path);
            self.status = format!("Using installed {}", entry.filename);
            return;
        }

        if self.install_rx.is_some() {
            self.status = "Wait for the current SoundFont install to finish".into();
            return;
        }

        let spec = InstallSpec {
            name: entry.name.to_string(),
            download_url: download_url.to_string(),
            filename: entry.filename.to_string(),
        };
        let (tx, rx) = crossbeam_channel::bounded(1);
        self.install_rx = Some(rx);
        self.installing_name = Some(entry.name.to_string());
        self.status = format!("Installing {}…", entry.name);

        std::thread::spawn(move || {
            let _ = tx.send(download_and_install(&spec));
        });
    }

    pub fn ui(&mut self, ui: &mut egui::Ui, studio: &StudioState) {
        self.poll_install();
        self.ensure_compatibility_report(studio);
        if self.install_rx.is_some() {
            ui.ctx().request_repaint_after(Duration::from_millis(100));
        }

        ui.heading("SoundFonts");
        ui.separator();

        if let Some(path) = self.missing_project_path.clone() {
            egui::Frame::group(ui.style())
                .fill(egui::Color32::from_rgb(58, 33, 33))
                .show(ui, |ui| {
                    ui.label(
                        egui::RichText::new("Project SoundFont missing")
                            .strong()
                            .color(egui::Color32::from_rgb(224, 108, 117)),
                    );
                    ui.label(
                        egui::RichText::new(path.display().to_string())
                            .small()
                            .monospace()
                            .color(egui::Color32::from_rgb(210, 210, 215)),
                    );
                    ui.label(
                        egui::RichText::new(
                            "Playback falls back to the oscillator. WAV export stays unavailable until you choose, install, or browse to a replacement.",
                        )
                        .small()
                        .color(egui::Color32::from_rgb(210, 210, 215)),
                    );
                    ui.horizontal(|ui| {
                        if ui.small_button("Clear project setting").clicked() {
                            self.clear_missing_reference();
                        }
                        if ui.small_button("Browse replacement…").clicked() {
                            self.browse_for_soundfont();
                        }
                    });
                });
            ui.separator();
        }

        // Active soundfont
        if let Some(ref path) = self.active_path {
            ui.label(
                egui::RichText::new(format!("Active: {}", path.display()))
                    .color(egui::Color32::from_rgb(152, 195, 121)),
            )
            .on_hover_text("The SoundFont used for playback and WAV export");
        } else {
            ui.label(
                egui::RichText::new("⚠ No SoundFont loaded — playback will use the oscillator fallback")
                    .color(egui::Color32::from_rgb(224, 108, 117)),
            );
        }

        if let Some(report) = &self.compatibility_report {
            ui.separator();
            if report.unsupported_format {
                ui.label(
                    egui::RichText::new("⚠ Current SoundFont format is unsupported by the playback engine")
                        .color(egui::Color32::from_rgb(224, 108, 117)),
                )
                .on_hover_text("The current backend is rustysynth, which supports .sf2 but not .sf3. Playback falls back and WAV export will fail for this file.");
            } else if report.issues.is_empty() {
                ui.label(
                    egui::RichText::new(format!("✓ SoundFont covers all project tracks ({} presets found)", report.preset_count))
                        .small()
                        .color(egui::Color32::from_rgb(152, 195, 121)),
                );
            } else {
                ui.label(
                    egui::RichText::new(format!("⚠ {} track(s) may not map cleanly in this SoundFont", report.issues.len()))
                        .small()
                        .color(egui::Color32::from_rgb(229, 192, 123)),
                );
                for issue in &report.issues {
                    let reason = match issue.reason {
                        TrackCompatibilityIssueKind::UnsupportedFormat => "unsupported SoundFont format",
                        TrackCompatibilityIssueKind::MissingPreset => "missing preset",
                    };
                    let suggestion = issue
                        .suggested_program
                        .map(|program| {
                            let name = issue
                                .suggested_name
                                .as_deref()
                                .unwrap_or("unknown");
                            format!("; suggested bank {} program {} ({})", issue.bank, program, name)
                        })
                        .unwrap_or_default();
                    ui.label(
                        egui::RichText::new(format!(
                            "• {} [{}] -> bank {} program {} ({}){}",
                            issue.track_name,
                            issue.instrument,
                            issue.bank,
                            issue.program,
                            reason,
                            suggestion,
                        ))
                        .small()
                        .color(egui::Color32::from_rgb(229, 192, 123)),
                    );
                }
            }
        }

        ui.separator();

        // Available soundfonts
        if self.available.is_empty() {
            ui.label(
                egui::RichText::new("No .sf2 / .sf3 files found on this system.")
                    .color(egui::Color32::from_rgb(180, 130, 60)),
            );
            ui.label(
                egui::RichText::new(format!(
                    "Place SoundFont files in: {}",
                    delphi_home().join("soundfonts").display()
                ))
                .small()
                .color(egui::Color32::from_rgb(150, 150, 150)),
            );
        } else {
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
                    if ui
                        .selectable_label(is_active, label)
                        .on_hover_text(entry.path.display().to_string())
                        .clicked()
                    {
                        set_active = Some(entry.path.clone());
                    }
                });
            }
            if let Some(path) = set_active {
                self.activate_path(path);
            }
        }

        ui.separator();

        // Browse and Refresh
        ui.horizontal(|ui| {
            if ui
                .button("Browse for .sf2…")
                .on_hover_text("Pick any .sf2 or .sf3 file on disk")
                .clicked()
            {
                self.browse_for_soundfont();
            }

            if ui
                .button("Refresh")
                .on_hover_text("Re-scan discovery paths")
                .clicked()
            {
                self.discover();
            }
        });

        ui.separator();

        // Curated catalog — get SoundFonts section
        let catalog_header = if self.show_catalog {
            "▼ Get SoundFonts"
        } else {
            "▶ Get SoundFonts"
        };
        if ui
            .button(catalog_header)
            .on_hover_text("Browse curated free SoundFonts with license info")
            .clicked()
        {
            self.show_catalog = !self.show_catalog;
        }

        if self.show_catalog {
            ui.separator();
            ui.label(
                egui::RichText::new(
                    "These SoundFonts are freely available with clear licensing.\n\
                     Delphi does not download files automatically — copy the URL\n\
                     and download it, then place it in ~/.delphi/soundfonts/.",
                )
                .small()
                .color(egui::Color32::from_rgb(150, 150, 150)),
            );
            ui.add_space(4.0);

            // Expire copy feedback after 2 seconds
            if let Some((_, t)) = &self.copy_feedback {
                if t.elapsed().as_secs() >= 2 {
                    self.copy_feedback = None;
                }
            }

            for entry in CATALOG {
                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(entry.name).strong());
                        ui.label(
                            egui::RichText::new(format!("[{}]", entry.license))
                                .small()
                                .color(egui::Color32::from_rgb(152, 195, 121)),
                        );
                    });
                    ui.label(
                        egui::RichText::new(entry.description)
                            .small()
                            .color(egui::Color32::from_rgb(180, 180, 190)),
                    );
                    ui.label(
                        egui::RichText::new(entry.install_note)
                            .small()
                            .color(egui::Color32::from_rgb(120, 120, 130)),
                    );
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new(entry.url)
                                .small()
                                .monospace()
                                .color(egui::Color32::from_rgb(86, 182, 194)),
                        );
                        let install_target = user_soundfont_dir().join(entry.filename);
                        let is_installing = self.installing_name.as_deref() == Some(entry.name);
                        if is_installing {
                            ui.add(egui::Spinner::new());
                            ui.label(
                                egui::RichText::new("Installing…")
                                    .small()
                                    .color(egui::Color32::from_rgb(229, 192, 123)),
                            );
                        } else if install_target.is_file() {
                            if ui.small_button("Use installed").clicked() {
                                self.activate_path(install_target);
                                self.status = format!("Using installed {}", entry.filename);
                            }
                        } else if entry.download_url.is_some() {
                            if ui.small_button("Install").clicked() {
                                self.start_install(entry);
                            }
                        } else {
                            ui.label(
                                egui::RichText::new("Manual only")
                                    .small()
                                    .color(egui::Color32::from_rgb(224, 108, 117)),
                            )
                            .on_hover_text(entry.install_note);
                        }
                        let copy_label =
                            if self.copy_feedback.as_ref().map_or(false, |(u, _)| u == entry.url) {
                                "✓ Copied"
                            } else {
                                "Copy URL"
                            };
                        if ui.small_button(copy_label).clicked() {
                            ui.ctx().copy_text(entry.url.to_string());
                            self.copy_feedback =
                                Some((entry.url.to_string(), Instant::now()));
                        }
                        ui.label(
                            egui::RichText::new(format!("→ save as: {}", entry.filename))
                                .small()
                                .color(egui::Color32::from_rgb(120, 120, 130)),
                        );
                    });
                });
                ui.add_space(2.0);
            }
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

    fn activate_path(&mut self, path: PathBuf) {
        self.missing_project_path = None;
        self.add_entry(&path);
        self.active_path = Some(path);
        self.compatibility_key = None;
    }

    fn browse_for_soundfont(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("SoundFont", &["sf2", "sf3"])
            .pick_file()
        {
            self.activate_path(path.clone());
            self.status = format!("Using {}", path.display());
        }
    }

    fn clear_missing_reference(&mut self) {
        self.missing_project_path = None;
        self.active_path = None;
        self.compatibility_report = None;
        self.compatibility_key = None;
        self.status = "Cleared missing project SoundFont reference".into();
    }

    pub fn set_active_path(&mut self, path: Option<PathBuf>) {
        match path {
            Some(path) if path.is_file() => {
                self.activate_path(path.clone());
                self.status = format!("Using {}", path.display());
            }
            Some(path) => {
                self.active_path = None;
                self.missing_project_path = Some(path.clone());
                self.compatibility_report = None;
                self.compatibility_key = None;
                self.status = format!("Project SoundFont missing: {}", path.display());
            }
            None => {
                self.active_path = None;
                self.missing_project_path = None;
                self.compatibility_report = None;
                self.compatibility_key = None;
                self.status = "No SoundFont selected".into();
            }
        }
    }

    pub fn missing_project_path(&self) -> Option<&Path> {
        self.missing_project_path.as_deref()
    }

    pub fn persisted_path(&self) -> Option<&Path> {
        self.missing_project_path()
            .or(self.active_path.as_deref())
    }

    pub fn compatibility_report(&self) -> Option<&SoundFontCompatibilityReport> {
        self.compatibility_report.as_ref()
    }

    pub fn refresh_compatibility(&mut self, studio: &StudioState) {
        self.ensure_compatibility_report(studio);
    }

    fn ensure_compatibility_report(&mut self, studio: &StudioState) {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        use std::hash::{Hash, Hasher};

        self.active_path.hash(&mut hasher);
        self.missing_project_path.hash(&mut hasher);
        for track in &studio.tracks {
            track.name.hash(&mut hasher);
            track.instrument.hash(&mut hasher);
            track.program.hash(&mut hasher);
            track.channel.hash(&mut hasher);
        }
        let key = hasher.finish();
        if self.compatibility_key == Some(key) {
            return;
        }
        self.compatibility_key = Some(key);

        self.compatibility_report = match self.active_path.as_deref() {
            Some(path) if path.is_file() => match audit_soundfont_compatibility(path, &studio.tracks) {
                Ok(report) => {
                    if report.unsupported_format {
                        self.status = "Warning: current SoundFont format is unsupported by the playback engine".into();
                    } else if report.issues.is_empty() {
                        self.status = format!("SoundFont covers all {} track(s)", studio.tracks.len());
                    } else {
                        self.status = format!(
                            "Warning: {} track(s) may not map in this SoundFont",
                            report.issues.len()
                        );
                    }
                    Some(report)
                }
                Err(err) => {
                    self.status = err.to_string();
                    None
                }
            },
            _ => None,
        };
    }
}

fn user_soundfont_dir() -> PathBuf {
    delphi_home().join("soundfonts")
}

fn download_and_install(spec: &InstallSpec) -> Result<InstallResult, String> {
    let install_dir = user_soundfont_dir();
    fs::create_dir_all(&install_dir)
        .map_err(|err| format!("could not create {}: {err}", install_dir.display()))?;

    let target_path = install_dir.join(&spec.filename);
    let temp_path = install_dir.join(format!("{}.download", spec.filename));
    let _ = fs::remove_file(&temp_path);

    let client = Client::builder()
        .user_agent("Delphi Studio")
        .build()
        .map_err(|err| format!("failed to start downloader: {err}"))?;

    let mut response = client
        .get(&spec.download_url)
        .send()
        .and_then(|response| response.error_for_status())
        .map_err(|err| format!("download request failed: {err}"))?;
    let content_type = response
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .to_ascii_lowercase();

    let mut temp_file = File::create(&temp_path)
        .map_err(|err| format!("could not create {}: {err}", temp_path.display()))?;
    std::io::copy(&mut response, &mut temp_file)
        .map_err(|err| format!("could not write {}: {err}", temp_path.display()))?;
    temp_file
        .flush()
        .map_err(|err| format!("could not finish {}: {err}", temp_path.display()))?;
    drop(temp_file);

    if is_zip_file(&temp_path)? {
        extract_soundfont_from_zip(&temp_path, &target_path)?;
        let _ = fs::remove_file(&temp_path);
    } else {
        if looks_like_html(&temp_path, &content_type)? {
            let _ = fs::remove_file(&temp_path);
            return Err("upstream returned a web page instead of a SoundFont file".into());
        }
        replace_file(&temp_path, &target_path)?;
    }

    Ok(InstallResult {
        installed_path: target_path.clone(),
        message: format!("Installed {} to {}", spec.name, target_path.display()),
    })
}

fn replace_file(from: &Path, to: &Path) -> Result<(), String> {
    if to.exists() {
        fs::remove_file(to).map_err(|err| format!("could not replace {}: {err}", to.display()))?;
    }
    fs::rename(from, to)
        .map_err(|err| format!("could not move {} into place: {err}", to.display()))
}

fn is_zip_file(path: &Path) -> Result<bool, String> {
    let mut file = File::open(path)
        .map_err(|err| format!("could not inspect {}: {err}", path.display()))?;
    let mut header = [0_u8; 4];
    let read = file
        .read(&mut header)
        .map_err(|err| format!("could not inspect {}: {err}", path.display()))?;
    Ok(read == 4 && header == *b"PK\x03\x04")
}

fn looks_like_html(path: &Path, content_type: &str) -> Result<bool, String> {
    if content_type.starts_with("text/html") {
        return Ok(true);
    }

    let mut file = File::open(path)
        .map_err(|err| format!("could not inspect {}: {err}", path.display()))?;
    let mut buf = [0_u8; 512];
    let read = file
        .read(&mut buf)
        .map_err(|err| format!("could not inspect {}: {err}", path.display()))?;
    let prefix = String::from_utf8_lossy(&buf[..read]).to_ascii_lowercase();
    Ok(prefix.contains("<!doctype html") || prefix.contains("<html") || prefix.contains("<head"))
}

fn extract_soundfont_from_zip(archive_path: &Path, target_path: &Path) -> Result<(), String> {
    let file = File::open(archive_path)
        .map_err(|err| format!("could not open {}: {err}", archive_path.display()))?;
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|err| format!("could not read {}: {err}", archive_path.display()))?;
    let preferred_ext = target_path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();

    let mut fallback_index = None;
    let mut preferred_index = None;

    for index in 0..archive.len() {
        let entry = archive
            .by_index(index)
            .map_err(|err| format!("could not inspect archive contents: {err}"))?;
        if !entry.is_file() {
            continue;
        }
        let name = entry.name().to_ascii_lowercase();
        if !(name.ends_with(".sf2") || name.ends_with(".sf3")) {
            continue;
        }
        if fallback_index.is_none() {
            fallback_index = Some(index);
        }
        if !preferred_ext.is_empty() && name.ends_with(&format!(".{preferred_ext}")) {
            preferred_index = Some(index);
            break;
        }
    }

    let Some(index) = preferred_index.or(fallback_index) else {
        return Err("downloaded archive did not contain an .sf2 or .sf3 file".into());
    };

    let mut source = archive
        .by_index(index)
        .map_err(|err| format!("could not read archive entry: {err}"))?;
    if target_path.exists() {
        fs::remove_file(target_path)
            .map_err(|err| format!("could not replace {}: {err}", target_path.display()))?;
    }
    let mut out = File::create(target_path)
        .map_err(|err| format!("could not create {}: {err}", target_path.display()))?;
    std::io::copy(&mut source, &mut out)
        .map_err(|err| format!("could not extract {}: {err}", target_path.display()))?;
    out.flush()
        .map_err(|err| format!("could not finish {}: {err}", target_path.display()))?;
    Ok(())
}

/// Get the Delphi config home: ~/.delphi/
fn delphi_home() -> PathBuf {
    if let Some(home) = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
    {
        PathBuf::from(home).join(".delphi")
    } else {
        PathBuf::from(".delphi")
    }
}

