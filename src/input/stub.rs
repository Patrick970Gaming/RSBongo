use super::AppEvent;
use crossbeam_channel::Sender;

/// No-op on non-Linux targets: evdev only exists on Linux. The window
/// still runs, it just won't react to input until a real backend
/// (e.g. Windows raw input / SetWindowsHookEx, macOS CGEventTap) is
/// written here.
pub fn spawn_listeners(_sender: Sender<AppEvent>) {
    eprintln!(
        "[input] no input backend for this platform yet — window will \
         render but won't react to key/mouse events."
    );
}
