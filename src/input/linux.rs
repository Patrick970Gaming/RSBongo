use super::AppEvent;
use crossbeam_channel::Sender;
use evdev::{Device, EventType, InputEventKind, Key};
use std::thread;

const IGNORED_KEYS: &[Key] = &[
    Key::BTN_TOUCH,
    Key::BTN_TOOL_FINGER,
    Key::BTN_TOOL_DOUBLETAP,
    Key::BTN_TOOL_TRIPLETAP,
    Key::BTN_TOOL_QUADTAP,
    Key::BTN_TOOL_QUINTTAP,
];

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

/// Spawns one reader thread per key-capable input device, each pushing
/// press/release events into the given channel.
pub fn spawn_listeners(sender: Sender<AppEvent>) {
    let devices = find_key_capable_devices();
    if devices.is_empty() {
        eprintln!("No key-capable devices found (or no permission to read them).");
        eprintln!("Fix: sudo usermod -aG input $USER   (then log out/in)");
        return;
    }

    for (name, mut device) in devices {
        let sender = sender.clone();

        thread::spawn(move || loop {
            match device.fetch_events() {
                Ok(events) => {
                    for event in events {
                        if let InputEventKind::Key(key) = event.kind() {
                            if IGNORED_KEYS.contains(&key) {
                                continue;
                            }
                            let app_event = match event.value() {
                                1 => Some(AppEvent::KeyPressed),
                                0 => Some(AppEvent::KeyReleased),
                                _ => None, // autorepeat / other
                            };
                            if let Some(ev) = app_event {
                                let _ = sender.send(ev);
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
}
