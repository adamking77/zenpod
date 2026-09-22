// Throwaway M0 probe for tauri-nspanel#123 (panel focusing like a window on macOS 27).
// Run with LISTENER_PANEL_TEST=regular|accessory|nokey|prevent. Delete once M6 builds the real Mini and Pill.
use tauri::{AppHandle, WebviewUrl, WebviewWindowBuilder};
use tauri_nspanel::{tauri_panel, Panel, PanelLevel, StyleMask, WebviewWindowExt};

tauri_panel! {
    panel!(KeyPanel { config: { can_become_key_window: true, is_floating_panel: true } })
    panel!(NoKeyPanel { config: { can_become_key_window: false, is_floating_panel: true } })
}

pub fn spawn(app: &AppHandle, variant: &str) -> tauri::Result<()> {
    if variant == "accessory" {
        app.set_activation_policy(tauri::ActivationPolicy::Accessory)?;
    }
    let window = WebviewWindowBuilder::new(app, "probe", WebviewUrl::App("index.html".into()))
        .title("Probe")
        .inner_size(280.0, 360.0)
        .position(200.0, 200.0)
        .decorations(false)
        .visible(false)
        .build()?;
    let panel: std::sync::Arc<dyn Panel> = if variant == "nokey" {
        window.to_panel::<NoKeyPanel>().expect("to_panel")
    } else {
        window.to_panel::<KeyPanel>().expect("to_panel")
    };
    panel.set_level(PanelLevel::Floating.value());
    panel
        .add_style_mask(StyleMask::empty().nonactivating_panel().into())
        .expect("nonactivating mask");
    if variant == "prevent" {
        // Private AppKit call: re-syncs the window-server "prevents activation" tag that a
        // post-init NonactivatingPanel mask change leaves stale.
        unsafe {
            let _: () = tauri_nspanel::objc2::msg_send![panel.as_panel(), _setPreventsActivation: true];
        }
    }
    let mask = panel.as_panel().styleMask();
    eprintln!("[probe] variant={variant} mask={mask:?} nonactivating={}", mask.0 & (1 << 7) != 0);

    let app = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_secs(8));
        let _ = app.run_on_main_thread(move || {
            panel.show();
            eprintln!("[probe] panel shown");
        });
    });
    Ok(())
}
