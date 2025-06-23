use eframe::NativeOptions;
use tel_ui::TelOnChainUI;

fn main() -> eframe::Result<()> {
    eframe::run_native(
        "Tel On Chain",
        NativeOptions::default(),
        Box::new(|cc| Box::new(TelOnChainUI::new(cc))),
    )
}
