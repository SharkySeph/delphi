mod app;
mod editor;
mod export;
mod mixer;
mod piano_roll;
mod player;
mod scripting;
mod soundfont;
mod studio;
mod theme;
mod theory;
mod visualizer;

fn main() -> eframe::Result {
    let icon_data = include_bytes!("../assets/icon.png");
    let icon_image = image::load_from_memory(icon_data).expect("embedded icon is valid PNG");
    let icon_rgba = icon_image.to_rgba8();
    let (w, h) = icon_rgba.dimensions();
    let icon = egui::IconData {
        rgba: icon_rgba.into_raw(),
        width: w,
        height: h,
    };

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Delphi Studio")
            .with_inner_size([1280.0, 800.0])
            .with_min_inner_size([800.0, 500.0])
            .with_icon(std::sync::Arc::new(icon)),
        ..Default::default()
    };

    eframe::run_native(
        "Delphi Studio",
        options,
        Box::new(|cc| Ok(Box::new(app::DelphiApp::new(cc)))),
    )
}
