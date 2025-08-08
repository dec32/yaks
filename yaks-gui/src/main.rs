use yaks_core::Engine;
slint::include_modules!();

fn main() {
    let ui = MainWindow::new().unwrap();
    let ui_handle = ui.as_weak();
    ui.on_download(move || {
        let _ui = ui_handle.unwrap();
        let _engine = Engine::default();
        println!("download start");
    });
    ui.run().unwrap();
}
