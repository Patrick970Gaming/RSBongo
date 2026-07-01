#[cfg(target_os = "linux")]
mod linux;
#[cfg(not(target_os = "linux"))]
mod stub;

#[cfg(target_os = "linux")]
pub use linux::spawn_listeners;
#[cfg(not(target_os = "linux"))]
pub use stub::spawn_listeners;

/// Events the input backend(s) push into the Bevy world via a
/// crossbeam channel. Platform-independent — every backend (evdev
/// today, eventually Windows/macOS equivalents) emits these.
#[derive(Debug, Clone, Copy)]
pub enum AppEvent {
    KeyPressed,
    KeyReleased,
}
