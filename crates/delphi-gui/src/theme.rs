use egui::{Color32, Visuals};

/// Delphi Studio's custom theme — dark with music-inspired accent colors.
#[allow(dead_code)]
pub struct DelphiTheme {
    pub bg_primary: Color32,
    pub bg_secondary: Color32,
    pub bg_editor: Color32,
    pub text_primary: Color32,
    pub text_secondary: Color32,
    pub accent_teal: Color32,
    pub accent_gold: Color32,
    pub accent_green: Color32,
    pub accent_red: Color32,
    pub accent_purple: Color32,
    pub accent_orange: Color32,
}

impl Default for DelphiTheme {
    fn default() -> Self {
        Self {
            bg_primary: Color32::from_rgb(30, 30, 35),
            bg_secondary: Color32::from_rgb(40, 42, 48),
            bg_editor: Color32::from_rgb(35, 37, 42),
            text_primary: Color32::from_rgb(200, 200, 210),
            text_secondary: Color32::from_rgb(140, 140, 150),
            accent_teal: Color32::from_rgb(86, 182, 194),
            accent_gold: Color32::from_rgb(229, 192, 123),
            accent_green: Color32::from_rgb(152, 195, 121),
            accent_red: Color32::from_rgb(224, 108, 117),
            accent_purple: Color32::from_rgb(198, 120, 221),
            accent_orange: Color32::from_rgb(209, 154, 102),
        }
    }
}

impl DelphiTheme {
    pub fn apply(&self, ctx: &egui::Context) {
        let mut visuals = Visuals::dark();

        // Window / panel backgrounds
        visuals.window_fill = self.bg_secondary;
        visuals.panel_fill = self.bg_primary;
        visuals.extreme_bg_color = self.bg_editor;

        // Accent color for selections, active widgets
        visuals.selection.bg_fill = Color32::from_rgb(56, 142, 154);
        visuals.selection.stroke.color = Color32::WHITE;

        // Hyperlinks
        visuals.hyperlink_color = self.accent_teal;

        // Widget colors
        visuals.widgets.noninteractive.bg_fill = self.bg_secondary;
        visuals.widgets.noninteractive.fg_stroke.color = self.text_primary;
        visuals.widgets.inactive.bg_fill = Color32::from_rgb(50, 52, 58);
        visuals.widgets.inactive.fg_stroke.color = self.text_secondary;
        visuals.widgets.hovered.bg_fill = Color32::from_rgb(60, 62, 70);
        visuals.widgets.hovered.fg_stroke.color = Color32::WHITE;
        visuals.widgets.active.bg_fill = Color32::from_rgb(70, 72, 80);
        visuals.widgets.active.fg_stroke.color = self.accent_teal;

        // Striped background for grids
        visuals.striped = true;

        ctx.set_visuals(visuals);

        // Set default fonts
        let fonts = egui::FontDefinitions::default();

        // Ensure monospace is available for the editor
        // (egui includes it by default, but we can customize later)

        ctx.set_fonts(fonts);
    }
}
