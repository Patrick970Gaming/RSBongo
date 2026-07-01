use serde::Deserialize;
use std::path::Path;

const CONFIG_PATH: &str = "config.toml";

/// Application config, loaded from config.toml in the working directory.
/// Deliberately minimal for now — this is the place to add future
/// settings (e.g. animation hold time, spritesheet path, server URL)
/// rather than hardcoding more constants in main.rs.
#[derive(Debug, Deserialize)]
pub struct Config {
    /// Scales the sprite image up or down. 1.0 = original spritesheet
    /// size, 2.0 = double size, 0.5 = half size. Must be > 0.
    #[serde(default = "default_scale")]
    pub scale: f32,

    /// Whether the overlay window should stay above other windows.
    /// Defaults to true, which is the whole point of an overlay — but
    /// useful to turn off for debugging (e.g. so you can drag it
    /// around without it fighting your window manager).
    #[serde(default = "default_always_on_top")]
    pub always_on_top: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            scale: default_scale(),
            always_on_top: default_always_on_top(),
        }
    }
}

fn default_scale() -> f32 {
    1.0
}

fn default_always_on_top() -> bool {
    true
}

/// Loads config.toml from the current working directory. If it doesn't
/// exist yet, writes out a default one (so there's something to edit)
/// and returns default values for this run. Falls back to defaults on
/// any parse/read error rather than failing the whole app over a bad
/// config file.
pub fn load() -> Config {
    let path = Path::new(CONFIG_PATH);

    if !path.exists() {
        let default = Config::default();
        let contents = format!(
            "# RSBongo configuration\n\n\
             # Scales the sprite image up or down. 1.0 = original size.\n\
             scale = {}\n\n\
             # Keep the overlay window above other windows.\n\
             always_on_top = {}\n",
            default.scale, default.always_on_top
        );

        match std::fs::write(path, contents) {
            Ok(()) => println!(
                "[config] wrote default config.toml (scale = {}, always_on_top = {})",
                default.scale, default.always_on_top
            ),
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
