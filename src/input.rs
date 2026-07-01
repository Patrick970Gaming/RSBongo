use evdev::{Device, EventType, InputEventKind, Key};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;
use winit::event_loop::EventLoopProxy;

/// Touchpad "contact" events, not actual clicks — fires constantly
/// just from resting/moving fingers on the pad. Real touchpad clicks
/// still come through as BTN_LEFT/BTN_RIGHT, which we keep.
const IGNORED_KEYS: &[Key] = &[
    Key::BTN_TOUCH,
    Key::BTN_TOOL_FINGER,
    Key::BTN_TOOL_DOUBLETAP,
    Key::BTN_TOOL_TRIPLETAP,
    Key::BTN_TOOL_QUADTAP,
    Key::BTN_TOOL_QUINTTAP,
];

/// Events the input thread(s) push up to the window/render thread.
#[derive(Debug, Clone, Copy)]
pub enum AppEvent {
    /// Any tracked key or button went down. This is what should
    /// trigger the bongo animation frame.
    KeyPressed,
    /// Any tracked key or button was released. This is what we'll
    /// eventually also use to fire an event to the server.
    KeyReleased,
}

fn find_key_capable_devices() -> Vec<(String, Device)> {
    let mut found = Vec::new();

    for (path, device) in evdev::enumerate() {
        let name = device.name().unwrap_or("Unknown device").to_string();

        if device.supported_events().contains(EventType::KEY) {
            println!("Found device: {} ({})", name, path.display());
            found.push((name, device));
        }
    }

    found
}

/// Called every time a tracked key/button is released.
///
/// Bumps the shared counter and forwards the event to the window so it
/// can react (animation now, server push later).
fn on_key_release(
    counter: &AtomicU64,
    proxy: &EventLoopProxy<AppEvent>,
    device_name: &str,
    key: Key,
) {
    let total = counter.fetch_add(1, Ordering::Relaxed) + 1;
    println!("[{device_name}] {key:?} RELEASED  (total releases: {total})");

    // TODO: send event to self-hosted server here
    let _ = proxy.send_event(AppEvent::KeyReleased);
}

/// Spawns one reader thread per key-capable input device. Returns the
/// shared release counter so the caller can inspect it later if needed.
pub fn spawn_listeners(proxy: EventLoopProxy<AppEvent>) -> Arc<AtomicU64> {
    let release_counter = Arc::new(AtomicU64::new(0));

    let devices = find_key_capable_devices();
    if devices.is_empty() {
        eprintln!("No key-capable devices found (or no permission to read them).");
        eprintln!("Fix: sudo usermod -aG input $USER   (then log out/in)");
        return release_counter;
    }

    for (name, mut device) in devices {
        let proxy = proxy.clone();
        let release_counter = Arc::clone(&release_counter);

        thread::spawn(move || loop {
            match device.fetch_events() {
                Ok(events) => {
                    for event in events {
                        if let InputEventKind::Key(key) = event.kind() {
                            if IGNORED_KEYS.contains(&key) {
                                continue;
                            }
                            match event.value() {
                                1 => {
                                    let _ = proxy.send_event(AppEvent::KeyPressed);
                                }
                                0 => on_key_release(&release_counter, &proxy, &name, key),
                                2 => continue, // autorepeat
                                _ => {}
                            }
                        }
                    }
                }
                Err(e) => {
                    eprintln!("[{name}] error reading events: {e}");
                    break;
                }
            }
        });
    }

    release_counter
}
