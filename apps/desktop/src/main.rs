mod editor;
use editor::Editor;
use eframe::egui;
use std::path::PathBuf;

fn main() -> eframe::Result {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let mut project = None;
    let mut capture = None;
    let mut i = 0;
    while i < args.len() {
        if args[i] == "--capture" {
            capture = args.get(i + 1).map(PathBuf::from);
            i += 2;
        } else {
            project = Some(PathBuf::from(&args[i]));
            i += 1;
        }
    }
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1500.0, 960.0])
            .with_min_inner_size([1120.0, 740.0])
            .with_title("Peachy Animator — First Slice"),
        renderer: eframe::Renderer::Wgpu,
        ..Default::default()
    };
    eframe::run_native(
        "Peachy Animator",
        options,
        Box::new(move |cc| Ok(Box::new(Editor::new(cc, project, capture)))),
    )
}
