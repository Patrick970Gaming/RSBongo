use evdev::{Device, EventType, InputEventKind, Key};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;

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

/// Opens every readable /dev/input/event* device that reports key events
/// (this covers both keyboards AND mice, since mouse buttons are also
/// EV_KEY events under evdev, e.g. BTN_LEFT/BTN_RIGHT).
fn find_key_capable_devices() -> Vec<(String, Device)> {
    let mut found = Vec::new();

    for (path, device) in evdev::enumerate() {
        let name = device.name().unwrap_or("Unknown device").to_string();

        let supports_keys = device
            .supported_events()
            .contains(EventType::KEY);

        if supports_keys {
            println!("Found device: {} ({})", name, path.display());
            found.push((name, device));
        }
    }

    found
}

/// Called every time a tracked key/button is released.
///
/// This is the hook point for the real app: right now it just bumps a
/// counter, but this is where we'll eventually trigger the bongo
/// animation frame and push an event to the self-hosted server.
fn on_key_release(counter: &AtomicU64, device_name: &str, key: Key) {
    let total = counter.fetch_add(1, Ordering::Relaxed) + 1;
    println!("[{device_name}] {key:?} RELEASED  (total releases: {total})");

    // TODO: trigger bongo cat animation frame here
    // TODO: send event to self-hosted server here
}

fn main() {
    println!("=== Input capture PoC ===");
    println!("This listens directly to /dev/input, so it will keep working");
    println!("even if this terminal/window is not focused.\n");

    let devices = find_key_capable_devices();

    if devices.is_empty() {
        eprintln!("\nNo key-capable devices found (or no permission to read them).");
        eprintln!("Fix: sudo usermod -aG input $USER   (then log out/in)");
        std::process::exit(1);
    }

    println!("\nListening... press keys or click your mouse. Ctrl+C to quit.\n");

    let release_counter = Arc::new(AtomicU64::new(0));
    let mut handles = Vec::new();

    for (name, mut device) in devices {
        let release_counter = Arc::clone(&release_counter);
        // one thread per device — fetch_events() blocks, so each device
        // needs its own reader
        let handle = thread::spawn(move || loop {
            match device.fetch_events() {
                Ok(events) => {
                    for event in events {
                        // value 1 = key/button down, 0 = up, 2 = autorepeat
                        if let InputEventKind::Key(key) = event.kind() {
                            if IGNORED_KEYS.contains(&key) {
                                continue;
                            }
                            match event.value() {
                                1 => println!("[{name}] {key:?} PRESSED"),
                                0 => on_key_release(&release_counter, &name, key),
                                2 => continue, // skip autorepeat spam
                                _ => println!("[{name}] {key:?} UNKNOWN"),
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
        handles.push(handle);
    }

    for handle in handles {
        let _ = handle.join();
    }
}
