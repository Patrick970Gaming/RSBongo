# input-poc

Minimal proof of concept: captures keyboard presses and mouse clicks
system-wide, regardless of window focus, by reading directly from
`/dev/input/event*` (evdev), bypassing X11/Wayland entirely.

## Setup

You need permission to read raw input devices without sudo:

```bash
sudo usermod -aG input $USER
```

Then **log out and back in** (group membership only applies to new
login sessions).

## Run

```bash
cargo build
cargo run
```

You should see a list of detected devices, then live PRESSED/RELEASED
events as you type or click — try switching focus to another window
and confirm it keeps printing.

## Notes / next steps for the real app

- This currently prints to stdout. For bongocat, replace the
  `println!` in the event loop with whatever triggers your animation
  state (e.g. send over an `mpsc::channel` to your render thread).
- Right now it opens *every* key-capable device it finds. In the real
  app you'll want to let the user pick their keyboard/mouse via
  `bongocat-find-devices`-style discovery, since laptops often expose
  multiple duplicate input nodes (lid switch, power button, etc. also
  show up as EV_KEY devices).
- No sudo required at runtime once the user is in the `input` group —
  important for a self-hosted/open-source tool, since you don't want
  to ask people to run a desktop pet as root.
- This works identically on X11 and Wayland since it never touches
  the display server — same code path you'll use for both.
