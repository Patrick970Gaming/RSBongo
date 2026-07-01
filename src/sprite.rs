use image::GenericImageView;

/// Which frame to display. Order here matches the horizontal layout
/// expected in the spritesheet PNG — see `SpriteSheet::load`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Frame {
    Idle = 0,
    LeftArmDown = 1,
    RightArmDown = 2,
}

const FRAME_COUNT: u32 = 3;

pub struct SpriteSheet {
    frame_width: u32,
    frame_height: u32,
    // one flat buffer per frame, each frame_width * frame_height u32s
    frames: Vec<Vec<u32>>,
}

impl SpriteSheet {
    /// Loads a spritesheet PNG expected to contain exactly `FRAME_COUNT`
    /// (3) equal-width frames side by side, left to right:
    ///   [ idle | left-arm-down | right-arm-down ]
    /// Frame width is inferred as `image_width / FRAME_COUNT`, so all
    /// three frames must be the same width. Frame height is the full
    /// image height.
    pub fn load(path: &str) -> Result<Self, image::ImageError> {
        let img = image::open(path)?;
        let (total_width, height) = img.dimensions();
        let frame_width = total_width / FRAME_COUNT;
        let rgba = img.to_rgba8();

        let mut frames = Vec::with_capacity(FRAME_COUNT as usize);
        for i in 0..FRAME_COUNT {
            let mut buf = vec![0u32; (frame_width * height) as usize];
            for y in 0..height {
                for x in 0..frame_width {
                    let px = rgba.get_pixel(i * frame_width + x, y);
                    let [r, g, b, a] = px.0;
                    // NOTE (unverified): packing as ARGB here. softbuffer's
                    // exact expected format (0RGB vs premultiplied ARGB)
                    // is the same open question flagged earlier re:
                    // transparency — if transparent pixels render as solid
                    // black/white instead of see-through, this packing is
                    // the first thing to revisit.
                    let packed = ((a as u32) << 24)
                        | ((r as u32) << 16)
                        | ((g as u32) << 8)
                        | (b as u32);
                    buf[(y * frame_width + x) as usize] = packed;
                }
            }
            frames.push(buf);
        }

        Ok(Self {
            frame_width,
            frame_height: height,
            frames,
        })
    }

    pub fn frame_size(&self) -> (u32, u32) {
        (self.frame_width, self.frame_height)
    }

    /// Copies the given frame directly into `dest`, which must be
    /// exactly `frame_width * frame_height` u32s (i.e. the window
    /// buffer should be sized to match the spritesheet frame size —
    /// see how the window is created in main.rs).
    pub fn draw(&self, dest: &mut [u32], frame: Frame) {
        let data = &self.frames[frame as usize];
        let len = data.len().min(dest.len());
        dest[..len].copy_from_slice(&data[..len]);
    }
}
