mod config;
mod input;

#[cfg(target_os = "linux")]
mod platform {
    pub mod x11;
}

use bevy::asset::AssetServer;
use bevy::input::ButtonInput;
use bevy::prelude::*;
use bevy::window::{PrimaryWindow, WindowLevel, WindowPosition, WindowResolution};
use bevy::winit::WinitWindows;
use input::AppEvent;
use std::time::Duration;

const SPRITESHEET_PATH: &str = "assets/bongocat.png";
const FRAME_COUNT: u32 = 3;

/// Which spritesheet column each state uses. Layout is
/// [ idle | left-arm-down | right-arm-down ], left to right.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Frame {
    Idle = 0,
    LeftArmDown = 1,
    RightArmDown = 2,
}

fn random_arm_frame() -> Frame {
    if rand::random::<bool>() {
        Frame::LeftArmDown
    } else {
        Frame::RightArmDown
    }
}

#[derive(Resource, Clone)]
struct AppConfig(config::Config);

#[derive(Resource)]
struct InputEventReceiver(crossbeam_channel::Receiver<AppEvent>);

#[derive(Resource, Default)]
struct ReleaseCounter(u64);

/// Marker + animation state for the cat sprite entity.
#[derive(Component, Default)]
struct BongoCat {
    revert_at: Option<f32>,
}

fn main() {
    println!("=== RSBongo (Bevy) ===");

    let cfg = config::load();

    // Peek the spritesheet's dimensions synchronously (just the header,
    // not a full decode) so we can size the window before Bevy's own
    // async asset loading has had a chance to load anything.
    let (total_width, height) = match image::image_dimensions(SPRITESHEET_PATH) {
        Ok(dims) => dims,
        Err(e) => {
            eprintln!("failed to read spritesheet at {SPRITESHEET_PATH}: {e}");
            eprintln!(
                "expected a PNG with 3 equal-width frames side by side: \
                 [ idle | left-arm-down | right-arm-down ]"
            );
            std::process::exit(1);
        }
    };
    let frame_width = total_width / FRAME_COUNT;

    let scaled_width = (frame_width as f32 * cfg.scale).round().max(1.0);
    let scaled_height = (height as f32 * cfg.scale).round().max(1.0);

    let window_level = if cfg.always_on_top {
        WindowLevel::AlwaysOnTop
    } else {
        WindowLevel::Normal
    };

    let window = Window {
        title: "RSBongo".into(),
        resolution: WindowResolution::new(scaled_width, scaled_height),
        transparent: true,
        decorations: false,
        resizable: false,
        window_level,
        position: WindowPosition::Automatic,
        ..default()
    };

    App::new()
        .insert_resource(AppConfig(cfg))
        .insert_resource(ReleaseCounter::default())
        .insert_resource(ClearColor(Color::NONE))
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(window),
            ..default()
        }))
        .add_systems(Startup, (setup, spawn_input_thread, setup_click_through))
        .add_systems(Update, (poll_input_events, handle_window_drag))
        .run();
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
    config: Res<AppConfig>,
) {
    commands.spawn(Camera2d);

    let (total_width, height) =
        image::image_dimensions(SPRITESHEET_PATH).expect("spritesheet already validated in main");
    let frame_width = total_width / FRAME_COUNT;

    let texture: Handle<Image> = asset_server.load(SPRITESHEET_PATH);
    let layout = TextureAtlasLayout::from_grid(
        UVec2::new(frame_width, height),
        FRAME_COUNT,
        1,
        None,
        None,
    );
    let layout_handle = atlas_layouts.add(layout);

    let scale = config.0.scale;

    commands.spawn((
        Sprite {
            image: texture,
            texture_atlas: Some(TextureAtlas {
                layout: layout_handle,
                index: Frame::Idle as usize,
            }),
            custom_size: Some(Vec2::new(frame_width as f32 * scale, height as f32 * scale)),
            ..default()
        },
        Transform::default(),
        BongoCat::default(),
    ));
}

/// Spawns the evdev listener thread(s) and hands the Bevy side a
/// receiver to poll every frame.
fn spawn_input_thread(mut commands: Commands) {
    let (sender, receiver) = crossbeam_channel::unbounded();
    input::spawn_listeners(sender);
    commands.insert_resource(InputEventReceiver(receiver));
}

/// Drains any pending input events, picks a random arm on press,
/// increments the release counter on release, and reverts to idle
/// after the configured hold time.
fn poll_input_events(
    receiver: Option<Res<InputEventReceiver>>,
    mut release_counter: ResMut<ReleaseCounter>,
    mut query: Query<(&mut Sprite, &mut BongoCat)>,
    time: Res<Time>,
    config: Res<AppConfig>,
) {
    let Some(receiver) = receiver else { return };
    let Ok((mut sprite, mut cat)) = query.single_mut() else {
        return;
    };

    for event in receiver.0.try_iter() {
        match event {
            AppEvent::KeyPressed => {
                if let Some(atlas) = sprite.texture_atlas.as_mut() {
                    atlas.index = random_arm_frame() as usize;
                }
                let hold_secs = Duration::from_millis(config.0.animation_hold_ms).as_secs_f32();
                cat.revert_at = Some(time.elapsed_secs() + hold_secs);
            }
            AppEvent::KeyReleased => {
                release_counter.0 += 1;
                // TODO: send event to self-hosted server here
            }
        }
    }

    if let Some(revert_at) = cat.revert_at {
        if time.elapsed_secs() >= revert_at {
            if let Some(atlas) = sprite.texture_atlas.as_mut() {
                atlas.index = Frame::Idle as usize;
            }
            cat.revert_at = None;
        }
    }
}

/// Lets you drag the overlay by clicking and holding — only actually
/// receives the click if `click_through = false` in config, since a
/// click-through window never gets mouse events in the first place
/// (they pass straight to whatever's underneath).
///
/// NOTE (unverified): WinitWindows + Window::drag_window() is the one
/// piece of this port I couldn't cross-check against an official Bevy
/// example — worth confirming this compiles/works first before relying
/// on it.
fn handle_window_drag(
    buttons: Res<ButtonInput<MouseButton>>,
    winit_windows: NonSend<WinitWindows>,
    primary_window: Query<Entity, With<PrimaryWindow>>,
) {
    if !buttons.just_pressed(MouseButton::Left) {
        return;
    }
    let Ok(entity) = primary_window.single() else {
        return;
    };
    let Some(window) = winit_windows.get_window(entity) else {
        return;
    };
    if let Err(e) = window.drag_window() {
        eprintln!("[drag] failed to start window drag: {e}");
    }
}

fn setup_click_through(
    winit_windows: NonSend<WinitWindows>,
    primary_window: Query<Entity, With<PrimaryWindow>>,
    config: Res<AppConfig>,
) {
    if !config.0.click_through {
        println!("[overlay] click_through = false — window is draggable, not click-through");
        return;
    }

    let Ok(entity) = primary_window.single() else {
        return;
    };
    let Some(_window) = winit_windows.get_window(entity) else {
        return;
    };

    #[cfg(target_os = "linux")]
    {
        if std::env::var("XDG_SESSION_TYPE").as_deref() == Ok("x11") {
            platform::x11::make_click_through(_window);
        } else {
            eprintln!(
                "[overlay] click-through skipped: not an X11 session \
                 (Wayland needs layer-shell, still on the TODO list)"
            );
        }
    }
}
