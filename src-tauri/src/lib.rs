#[cfg(target_os = "macos")]
mod panel_test;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default();
    #[cfg(target_os = "macos")]
    let builder = builder.plugin(tauri_nspanel::init()).setup(|app| {
        if let Ok(v) = std::env::var("LISTENER_PANEL_TEST") {
            panel_test::spawn(app.handle(), &v)?;
        }
        Ok(())
    });
    builder
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
