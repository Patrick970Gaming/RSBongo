<<<<<<< HEAD
# RSBongo
=======
# BongoRS— bongocat overlay proof of concept
>>>>>>> dev

Global keyboard/mouse capture (evdev) driving a transparent, always-on-top
Bevy window with a sprite that reacts to input, regardless of window focus.

Ported from a hand-rolled winit + softbuffer version to Bevy 0.17 for:
- real alpha-blended transparency (wgpu, instead of guessing at softbuffer's
  pixel format)
- a proper game loop instead of manually managing winit's ControlFlow
- `window.drag_window()` for moving the overlay around, which the old
  version had no path to at all

## Setup

```bash
sudo usermod -aG input $USER
# log out and back in
```

## Config

`config.toml` is created automatically on first run if missing:

```toml
scale = 1.0
always_on_top = true
click_through = true
animation_hold_ms = 60
```

Set `click_through = false` while positioning the cat — with click-through
on, the window physically cannot receive the mouse clicks needed to drag
it (they pass straight through to whatever's underneath). Flip it back to
`true` once you're happy with placement.

## Spritesheet

Same contract as before: `assets/bongocat.png`, 3 equal-width frames side
by side, `[ idle | left-arm-down | right-arm-down ]`. Not included — supply
your own.

## Run

```bash
cargo build
cargo run
```

## Known unverified spots

1. **Window dragging (`handle_window_drag` in main.rs)** — uses
   `bevy::winit::WinitWindows` + `winit::window::Window::drag_window()`.
   This is the one part of the port I couldn't cross-check against an
   official Bevy example. If it doesn't compile, the fix is likely a
   renamed resource/method between Bevy patch versions — check Bevy's
   migration guide for the version you land on.
2. **`Query::single_mut()` / `single()`** — Bevy has renamed and
   re-signatured these (`get_single` vs `single`, panic vs `Result`)
   across several releases. Written here assuming `single()` returns
   `Result`, matching Bevy 0.17. If your resolved version differs,
   expect a small signature mismatch here.
3. **Startup system ordering** — `setup_click_through` and
   `handle_window_drag` assume the primary window already exists in
   `WinitWindows` by the time they run. This *should* hold since
   `bevy_winit` creates the window before `Startup` systems run, but
   hasn't been verified against a live build.
4. **X11 click-through** — same as before, untested against a live X
   server.
5. **Crate version drift generally** — Bevy 0.17 is what this targets
   (0.18 is still RC as of writing). If `cargo build` resolves a newer
   Bevy, check the migration guide first for anything that doesn't
   compile.

## What's next

- Wayland/wlroots support via `gtk4-layer-shell` or `smithay-client-toolkit`
  (still no path around GNOME's lack of layer-shell support)
- A runtime hotkey to toggle `click_through` without editing config.toml
  and restarting — right now it's edit-and-relaunch only
- Wire `TODO: send event to self-hosted server here` in `poll_input_events`
  into a real HTTP/WebSocket client once the server exists
