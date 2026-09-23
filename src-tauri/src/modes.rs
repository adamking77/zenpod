//! The Mini and the Pill: normal-level panels that never take focus from the app you're in,
//! on the desktop's frosted material. The main window, Mini and Pill show one at a time.

use tauri::{AppHandle, Emitter, LogicalPosition, LogicalSize, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder};
use tauri_nspanel::{tauri_panel, CollectionBehavior, ManagerExt, PanelLevel, StyleMask, WebviewWindowExt};
use window_vibrancy::{apply_vibrancy, NSVisualEffectMaterial, NSVisualEffectState};

use crate::{err, store, Core, R};

tauri_panel! {
    panel!(Floating { config: { can_become_key_window: true, is_floating_panel: false } })
}

pub const MINI: (f64, f64) = (280.0, 404.0);
pub const PILL: (f64, f64) = (340.0, 48.0);
pub const PANEL: (f64, f64) = (340.0, 184.0);
const GAP: f64 = 10.0;

fn build(app: &AppHandle, label: &str, (w, h): (f64, f64), radius: f64) -> tauri::Result<WebviewWindow> {
    let win = WebviewWindowBuilder::new(app, label, WebviewUrl::App(label.into()))
        .title("Zenpod")
        .inner_size(w, h)
        .resizable(false)
        .decorations(false)
        .transparent(true)
        .shadow(true)
        .skip_taskbar(true)
        .accept_first_mouse(true)
        .visible(false)
        .build()?;
    let _ = apply_vibrancy(&win, NSVisualEffectMaterial::Popover, Some(NSVisualEffectState::Active), Some(radius));
    let panel = win.to_panel::<Floating>().map_err(|e| tauri::Error::Io(anyhow_like(e)))?;
    panel.set_level(PanelLevel::Normal.value());
    // Joining the app never activates it: the mask alone isn't enough on macOS 27 (tauri-nspanel#123),
    // the private flag re-syncs the window server's activation tag. Verified in M0.
    let _ = panel.add_style_mask(StyleMask::empty().nonactivating_panel().into());
    unsafe {
        let _: () = tauri_nspanel::objc2::msg_send![panel.as_panel(), _setPreventsActivation: true];
    }
    panel.set_collection_behavior(CollectionBehavior::new().can_join_all_spaces().full_screen_auxiliary().into());
    panel.set_hides_on_deactivate(false);
    Ok(win)
}

fn anyhow_like(e: impl std::fmt::Debug) -> std::io::Error {
    std::io::Error::other(format!("{e:?}"))
}

/// Create the three hidden surfaces. First run places them; later runs restore where they were left.
pub fn spawn(app: &AppHandle) -> tauri::Result<()> {
    let mini = build(app, "mini", MINI, 18.0)?;
    let pill = build(app, "pill", PILL, PILL.1 / 2.0)?;
    build(app, "panel", PANEL, 18.0)?;
    if let Ok(Some(m)) = mini.current_monitor() {
        let s = m.size().to_logical::<f64>(m.scale_factor());
        let first = store::setting(&app.state::<Core>().db.lock().unwrap(), "placed").is_none();
        if first {
            let _ = mini.set_position(LogicalPosition::new(s.width - MINI.0 - 80.0, 64.0));
            let _ = pill.set_position(LogicalPosition::new((s.width - PILL.0) / 2.0, s.height - PILL.1 - 44.0));
            let _ = store::set_setting(&app.state::<Core>().db.lock().unwrap(), "placed", "1");
        }
    }
    Ok(())
}

fn show_panel(app: &AppHandle, label: &str) {
    if let Ok(p) = app.get_webview_panel(label) {
        p.show();
    }
}

fn hide_panel(app: &AppHandle, label: &str) {
    if let Ok(p) = app.get_webview_panel(label) {
        p.hide();
    }
}

/// Switch between the window, the Mini and the Pill.
#[tauri::command]
pub fn set_mode(app: AppHandle, to: String) -> R<()> {
    let main = app.get_webview_window("main").ok_or("no main window")?;
    match to.as_str() {
        "mini" => {
            main.hide().map_err(err)?;
            hide_panel(&app, "pill");
            hide_panel(&app, "panel");
            show_panel(&app, "mini");
        }
        "pill" => {
            main.hide().map_err(err)?;
            hide_panel(&app, "mini");
            show_panel(&app, "pill");
        }
        _ => {
            hide_panel(&app, "mini");
            hide_panel(&app, "pill");
            hide_panel(&app, "panel");
            main.show().map_err(err)?;
            main.set_focus().map_err(err)?;
        }
    }
    let _ = app.emit("mode", &to);
    Ok(())
}

/// The Pill's panel opens upward from the Pill, only when asked.
#[tauri::command]
pub fn pill_panel(app: AppHandle, open: bool) -> R<()> {
    if !open {
        hide_panel(&app, "panel");
        let _ = app.emit("panel", false);
        return Ok(());
    }
    let pill = app.get_webview_window("pill").ok_or("no pill")?;
    let panel = app.get_webview_window("panel").ok_or("no panel")?;
    let scale = pill.scale_factor().map_err(err)?;
    let at = pill.outer_position().map_err(err)?.to_logical::<f64>(scale);
    panel.set_size(LogicalSize::new(PANEL.0, PANEL.1)).map_err(err)?;
    panel.set_position(LogicalPosition::new(at.x, at.y - PANEL.1 - GAP)).map_err(err)?;
    show_panel(&app, "panel");
    let _ = app.emit("panel", true);
    Ok(())
}

/// Carry a Mini or Pill (or the Pill by its panel) with the pointer from the press the page reports until the button comes up,
/// once it has travelled a few points. The page can't do this itself: a panel that never activates
/// the app gets no usable pointer moves. Answers whether it was a drag, so the page can drop the click.
#[tauri::command]
pub async fn drag_panel(app: AppHandle, window: WebviewWindow) -> R<bool> {
    use tauri_nspanel::objc2_app_kit::NSEvent;
    // The Pill's panel carries its Pill, and follows it like any other move.
    let window = if window.label() == "panel" { app.get_webview_window("pill").ok_or("no pill")? } else { window };
    let scale = window.scale_factor().map_err(err)?;
    let at = window.outer_position().map_err(err)?.to_logical::<f64>(scale);
    let from = NSEvent::mouseLocation();
    let (mut dragged, mut last) = (false, (0.0, 0.0));
    while NSEvent::pressedMouseButtons() & 1 == 1 {
        let p = NSEvent::mouseLocation();
        // Screen y runs upward in AppKit and downward in window positions.
        let d = (p.x - from.x, from.y - p.y);
        if d != last && (dragged || d.0.hypot(d.1) >= 4.0) {
            dragged = true;
            last = d;
            window.set_position(LogicalPosition::new(at.x + d.0, at.y + d.1)).map_err(err)?;
        }
        tokio::time::sleep(std::time::Duration::from_millis(8)).await;
    }
    Ok(dragged)
}

