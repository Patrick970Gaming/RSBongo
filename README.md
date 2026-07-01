# BongoRS— bongocat overlay proof of concept

Global keyboard/mouse capture (evdev) driving a transparent always-on-top
window with a sprite that reacts to input, regardless of window focus.

## Config

`config.toml` is created automatically on first run (in the working
directory) if it doesn't exist yet, with default values. Currently just:

```toml
# RSBongo configuration

# Scales the sprite image up or down. 1.0 = original size.
scale = 1.0

# Keep the overlay window above other windows.
always_on_top = true
```

More settings (animation timing, spritesheet path, server URL, etc.) will
land here over time — this is the intended home for future config rather
than hardcoding more constants in main.rs.

## Spritesheet

You need to supply `assets/bongocat.png` yourself — it's not included.
Layout requirement: **3 equal-width frames side by side, left to right**:

```
[ idle | left-arm-down | right-arm-down ]
```

Frame width is inferred as `total_image_width / 3`, so all three frames
must be the same width; frame height is just the full image height. The
window is sized to exactly one frame, so keep the cat centered within
each frame's bounds.

Each keypress/release randomly picks left or right arm (50/50) via `rand`,
holds it for `ANIMATION_HOLD`, then reverts to idle.

## Setup

```bash
sudo usermod -aG input $USER
# log out and back in
```

## Run

```bash
cargo build
cargo run
```

A small window should appear near the bottom-center of your screen, sized to
match your spritesheet's frame dimensions. Every key press/release should
briefly show a randomly-chosen arm-down frame, then snap back to idle.

## Known unverified spots (couldn't network-build to check these)

1. **softbuffer transparency** — plain `softbuffer` buffers are typically
   0RGB with no real alpha channel on most backends. `with_transparent(true)`
   on the winit window is necessary but may not be sufficient on its own to
   get a see-through background; if the window shows solid black instead of
   transparent, this is the first thing to dig into (may need a different
   rendering backend, e.g. `wgpu`, for real alpha compositing).
2. **X11 click-through (`src/platform/x11.rs`)** — written against `x11rb`'s
   SHAPE extension API from memory, not tested against a live X server.
   Verify `conn.shape_mask(...)` signature matches your resolved `x11rb`
   version; the crate's API has shifted across versions before.
3. **Crate version drift** — `winit` 0.30's `ApplicationHandler` API and
   `softbuffer` 0.4 are what this is written against. If `cargo build` pulls
   different minor versions with breaking changes, check each crate's
   CHANGELOG for the relevant method renames.

## What's next

- Wayland/wlroots support via `gtk4-layer-shell` or `smithay-client-toolkit`
  (GNOME/Wayland will keep falling back to a plain non-click-through window,
  as discussed — no way around that without a compositor extension)
- Distinguish keyboard vs mouse events if you want different animations
- Wire the `TODO: send event to self-hosted server here` in `src/input.rs`
  into an actual HTTP/WebSocket client once the server exists
