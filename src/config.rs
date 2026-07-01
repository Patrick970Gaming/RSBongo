use serde::Deserialize;
use std::path::Path;

const CONFIG_PATH: &str = "config.toml";

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    /// Scales the sprite up or down. 1.0 = original spritesheet size.
    #[serde(default = "default_scale")]
    pub scale: f32,

    /// Whether the overlay window should stay above other windows.
    #[serde(default = "default_always_on_top")]
    pub always_on_top: bool,

    /// Whether clicks pass through the window to whatever's underneath.
    /// Set this to `false` while developing/positioning the cat — with
    /// click-through on, the window can never receive the mouse clicks
    /// needed to drag it around.
    #[serde(default = "default_click_through")]
    pub click_through: bool,

    /// Milliseconds the "arm down" frame stays up before reverting to idle.
    #[serde(default = "default_animation_hold_ms")]
    pub animation_hold_ms: u64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            scale: default_scale(),
            always_on_top: default_always_on_top(),
            click_through: default_click_through(),
            animation_hold_ms: default_animation_hold_ms(),
        }
    }
}

fn default_scale() -> f32 {
    1.0
}
fn default_always_on_top() -> bool {
    true
}
fn default_click_through() -> bool {
    true
}
fn default_animation_hold_ms() -> u64 {
    60
}

/// Loads config.toml from the working directory, creating a default one
/// on first run. Falls back to defaults on any parse/read error.
pub fn load() -> Config {
    let path = Path::new(CONFIG_PATH);

    if !path.exists() {
        let default = Config::default();
        let contents = format!(
            "# RSBongo configuration\n\n\
             # Scales the sprite image up or down. 1.0 = original size.\n\
             scale = {}\n\n\
             # Keep the overlay window above other windows.\n\
             always_on_top = {}\n\n\
             # Set to false to be able to click and drag the window\n\
             # (e.g. while positioning it). true = normal overlay behavior.\n\
             click_through = {}\n\n\
             # How long the arm-down frame stays visible per tap, in ms.\n\
             animation_hold_ms = {}\n",
            default.scale, default.always_on_top, default.click_through, default.animation_hold_ms
        );

        match std::fs::write(path, contents) {
            Ok(()) => println!("[config] wrote default config.toml"),
            Err(e) => eprintln!("[config] couldn't write default config.toml: {e}"),
        }

        return default;
    }

    let contents = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("[config] failed to read config.toml: {e}, using defaults");
            return Config::default();
        }
    };

    match toml::from_str::<Config>(&contents) {
        Ok(cfg) if cfg.scale > 0.0 => cfg,
        Ok(_) => {
            eprintln!("[config] scale must be > 0, falling back to default");
            Config::default()
        }
        Err(e) => {
            eprintln!("[config] failed to parse config.toml: {e}, using defaults");
            Config::default()
        }
    }
}
