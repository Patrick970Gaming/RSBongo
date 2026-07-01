use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use x11rb::connection::Connection;
use x11rb::protocol::shape::{self, ConnectionExt as _};
use x11rb::rust_connection::RustConnection;

/// Makes the window fully click-through on X11 via the XShape extension.
/// Generic over `HasWindowHandle` rather than naming `winit::window::Window`
/// directly — we don't depend on `winit` ourselves (Bevy pulls it in
/// internally), so naming its concrete type here risked a version
/// mismatch. Works with whatever window type Bevy hands us, as long as
/// it implements the same raw-window-handle trait (it does).
///
/// NOTE (unverified): not tested against a live X server in this
/// environment — same caveat as before.
pub fn make_click_through(window: &impl HasWindowHandle) {
    let win_id = match window.window_handle().map(|h| h.as_raw()) {
        Ok(RawWindowHandle::Xlib(handle)) => handle.window as u32,
        Ok(RawWindowHandle::Xcb(handle)) => handle.window.get(),
        _ => {
            eprintln!("[x11] not an X11 window handle, skipping click-through");
            return;
        }
    };

    let (conn, _screen_num) = match RustConnection::connect(None) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("[x11] couldn't connect to X server for click-through: {e}");
            return;
        }
    };

    let empty_region = x11rb::NONE;

    if let Err(e) = conn.shape_mask(shape::SO::SET, shape::SK::INPUT, win_id, 0, 0, empty_region) {
        eprintln!("[x11] failed to set click-through shape: {e}");
        return;
    }

    if let Err(e) = conn.flush() {
        eprintln!("[x11] failed to flush X connection: {e}");
        return;
    }

    println!("[x11] click-through enabled");
}
