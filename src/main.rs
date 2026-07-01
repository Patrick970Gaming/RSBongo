mod input;
mod sprite;

#[cfg(target_os = "linux")]
mod platform {
    pub mod x11;
}

use input::AppEvent;
use rand::Rng;
use sprite::{Frame, SpriteSheet};
use std::num::NonZeroU32;
use std::sync::Arc;
use std::time::{Duration, Instant};
use winit::application::ApplicationHandler;
use winit::dpi::{LogicalPosition, LogicalSize};
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop, EventLoopProxy};
use winit::window::{Window, WindowAttributes, WindowLevel};

// path to the spritesheet PNG — 3 equal-width frames side by side:
// [ idle | left-arm-down | right-arm-down ]. See sprite.rs for the
// exact layout contract.
const SPRITESHEET_PATH: &str = "assets/bongocat.png";

// how long the "hit" frame stays up after a keypress before reverting to idle
// tune this to taste — lower = snappier, but too low and fast typing may
// look like a constant blur rather than distinct taps
const ANIMATION_HOLD: Duration = Duration::from_millis(60);

struct App {
    sheet: SpriteSheet,
    frame_width: u32,
    frame_height: u32,
    window: Option<Arc<Window>>,
    surface: Option<softbuffer::Surface<Arc<Window>, Arc<Window>>>,
    current_frame: Frame,
    revert_at: Option<Instant>,
}

impl App {
    fn new(sheet: SpriteSheet) -> Self {
        let (frame_width, frame_height) = sheet.frame_size();
        Self {
            sheet,
            frame_width,
            frame_height,
            window: None,
            surface: None,
            current_frame: Frame::Idle,
            revert_at: None,
        }
    }

    fn redraw(&mut self) {
        let Some(surface) = self.surface.as_mut() else { return };
        let mut buffer = match surface.buffer_mut() {
            Ok(b) => b,
            Err(e) => {
                eprintln!("failed to get draw buffer: {e}");
                return;
            }
        };

        self.sheet.draw(&mut buffer, self.current_frame);

        if let Err(e) = buffer.present() {
            eprintln!("failed to present frame: {e}");
        }
    }

    /// Picks a random arm to animate — this is the "individual control
    /// of the arms" behavior: each event independently rolls which
    /// side taps, rather than always alternating or always using both.
    fn random_arm_frame() -> Frame {
        if rand::thread_rng().gen_bool(0.5) {
            Frame::LeftArmDown
        } else {
            Frame::RightArmDown
        }
    }
}

impl ApplicationHandler<AppEvent> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return; // already set up
        }

        let width = self.frame_width;
        let height = self.frame_height;

        // figure out a bottom-of-screen position on the primary monitor,
        // falling back to (100, 100) if we can't detect one
        let position = event_loop
            .primary_monitor()
            .map(|m| {
                let size = m.size();
                LogicalPosition::new(
                    (size.width as f64 / 2.0) - (width as f64 / 2.0),
                    size.height as f64 - height as f64 - 40.0,
                )
            })
            .unwrap_or(LogicalPosition::new(100.0, 100.0));

        let attrs = WindowAttributes::default()
            .with_inner_size(LogicalSize::new(width, height))
            .with_position(position)
            .with_decorations(false)
            .with_transparent(true)
            .with_resizable(false)
            .with_window_level(WindowLevel::AlwaysOnTop)
            .with_title("RSBongo");

        let window = Arc::new(
            event_loop
                .create_window(attrs)
                .expect("failed to create window"),
        );

        #[cfg(target_os = "linux")]
        {
            // only meaningful under X11 — see platform::x11 for details
            if std::env::var("XDG_SESSION_TYPE").as_deref() == Ok("x11") {
                platform::x11::make_click_through(&window);
            } else {
                eprintln!(
                    "[overlay] click-through skipped: not an X11 session \
                     (Wayland needs layer-shell, which isn't wired up yet)"
                );
            }
        }

        let context = softbuffer::Context::new(Arc::clone(&window)).expect("softbuffer context");
        let mut surface =
            softbuffer::Surface::new(&context, Arc::clone(&window)).expect("softbuffer surface");
        surface
            .resize(
                NonZeroU32::new(width).unwrap(),
                NonZeroU32::new(height).unwrap(),
            )
            .expect("failed to size surface");

        self.window = Some(window);
        self.surface = Some(surface);
        self.redraw();
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: AppEvent) {
        match event {
            AppEvent::KeyPressed | AppEvent::KeyReleased => {
                self.current_frame = Self::random_arm_frame();
                self.revert_at = Some(Instant::now() + ANIMATION_HOLD);
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
                event_loop.set_control_flow(ControlFlow::WaitUntil(
                    Instant::now() + ANIMATION_HOLD,
                ));
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::RedrawRequested => self.redraw(),
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if let Some(deadline) = self.revert_at {
            if Instant::now() >= deadline {
                self.current_frame = Frame::Idle;
                self.revert_at = None;
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
                event_loop.set_control_flow(ControlFlow::Wait);
            } else {
                event_loop.set_control_flow(ControlFlow::WaitUntil(deadline));
            }
        }
    }
}

fn main() {
    println!("=== RSBongo overlay PoC ===");

    let sheet = match SpriteSheet::load(SPRITESHEET_PATH) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("failed to load spritesheet at {SPRITESHEET_PATH}: {e}");
            eprintln!(
                "expected a PNG with 3 equal-width frames side by side: \
                 [ idle | left-arm-down | right-arm-down ]"
            );
            std::process::exit(1);
        }
    };

    let event_loop: EventLoop<AppEvent> =
        EventLoop::with_user_event().build().expect("event loop");
    let proxy: EventLoopProxy<AppEvent> = event_loop.create_proxy();

    input::spawn_listeners(proxy);

    let mut app = App::new(sheet);
    event_loop.run_app(&mut app).expect("event loop run");
}
