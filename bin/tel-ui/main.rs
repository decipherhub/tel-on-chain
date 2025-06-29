fn main() -> eframe::Result<()> {
    tracing_subscriber::fmt::init();
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1000.0, 800.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Tel-On-Chain Debug UI",
        options,
        Box::new(|cc| Box::new(tel_ui::TelOnChainUI::new(cc))),
    )
}
