mod input;
mod sprite;

#[cfg(target_os = "linux")]
mod platform {
    pub mod x11;
}

use input::AppEvent;
use std::num::NonZeroU32;
use std::sync::Arc;
use std::time::{Duration, Instant};
use winit::application::ApplicationHandler;
use winit::dpi::{LogicalPosition, LogicalSize};
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop, EventLoopProxy};
use winit::window::{Window, WindowAttributes, WindowLevel};

const WINDOW_WIDTH: u32 = 200;
const WINDOW_HEIGHT: u32 = 100;
// how long the "hit" frame stays up after a keypress before reverting to idle
const ANIMATION_HOLD: Duration = Duration::from_millis(150);

struct App {
    window: Option<Arc<Window>>,
    surface: Option<softbuffer::Surface<Arc<Window>, Arc<Window>>>,
    active: bool,
    revert_at: Option<Instant>,
}

impl App {
    fn new() -> Self {
        Self {
            window: None,
            surface: None,
            active: false,
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

        sprite::draw(&mut buffer, WINDOW_WIDTH, WINDOW_HEIGHT, self.active);

        if let Err(e) = buffer.present() {
            eprintln!("failed to present frame: {e}");
        }
    }
}

impl ApplicationHandler<AppEvent> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return; // already set up
        }

        // figure out a bottom-of-screen position on the primary monitor,
        // falling back to (100, 100) if we can't detect one
        let position = event_loop
            .primary_monitor()
            .map(|m| {
                let size = m.size();
                LogicalPosition::new(
                    (size.width as f64 / 2.0) - (WINDOW_WIDTH as f64 / 2.0),
                    size.height as f64 - WINDOW_HEIGHT as f64 - 40.0,
                )
            })
            .unwrap_or(LogicalPosition::new(100.0, 100.0));

        let attrs = WindowAttributes::default()
            .with_inner_size(LogicalSize::new(WINDOW_WIDTH, WINDOW_HEIGHT))
            .with_position(position)
            .with_decorations(false)
            .with_transparent(true)
            .with_resizable(false)
            .with_window_level(WindowLevel::AlwaysOnTop)
            .with_title("bongocat-poc");

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
                NonZeroU32::new(WINDOW_WIDTH).unwrap(),
                NonZeroU32::new(WINDOW_HEIGHT).unwrap(),
            )
            .expect("failed to size surface");

        self.window = Some(window);
        self.surface = Some(surface);
        self.redraw();
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: AppEvent) {
        match event {
            AppEvent::KeyPressed | AppEvent::KeyReleased => {
                self.active = true;
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
                self.active = false;
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
    println!("=== bongocat overlay PoC ===");

    let event_loop: EventLoop<AppEvent> =
        EventLoop::with_user_event().build().expect("event loop");
    let proxy: EventLoopProxy<AppEvent> = event_loop.create_proxy();

    input::spawn_listeners(proxy);

    let mut app = App::new();
    event_loop.run_app(&mut app).expect("event loop run");
}
