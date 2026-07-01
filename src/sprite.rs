/// Draws one frame into `buffer` (a `width * height` array of 0RGB u32
/// pixels, as expected by softbuffer). This is a placeholder sprite —
/// two crude drawn frames — so the PoC has zero external asset
/// dependencies. Swap this out for real spritesheet rendering later.
pub fn draw(buffer: &mut [u32], width: u32, height: u32, active: bool) {
    let width = width as i32;
    let height = height as i32;

    // Fully transparent background. NOTE: plain softbuffer buffers are
    // 0RGB (no alpha channel) on most backends, so true per-pixel
    // transparency depends on the window compositor picking up alpha
    // from elsewhere. This line is a no-op placeholder for that;
    // expect to revisit this once you can see it rendered — it's one
    // of the two things flagged as unverified in the writeup.
    buffer.fill(0x00000000);

    let cx = width / 2;
    let body_y = height - 30;

    // Body (simple circle-ish blob via a filled rect for now)
    fill_rect(buffer, width, height, cx - 25, body_y - 20, 50, 20, 0xFFDDA85F);

    // Ears
    fill_rect(buffer, width, height, cx - 22, body_y - 30, 8, 10, 0xFFDDA85F);
    fill_rect(buffer, width, height, cx + 14, body_y - 30, 8, 10, 0xFFDDA85F);

    // Paws — this is the part that moves between frames
    let paw_y = if active { body_y - 35 } else { body_y - 15 };
    fill_rect(buffer, width, height, cx - 30, paw_y, 10, 10, 0xFF553311);
    fill_rect(buffer, width, height, cx + 20, paw_y, 10, 10, 0xFF553311);
}

fn fill_rect(buffer: &mut [u32], width: i32, height: i32, x: i32, y: i32, w: i32, h: i32, color: u32) {
    for row in y.max(0)..(y + h).min(height) {
        for col in x.max(0)..(x + w).min(width) {
            let idx = (row * width + col) as usize;
            if idx < buffer.len() {
                buffer[idx] = color;
            }
        }
    }
}
