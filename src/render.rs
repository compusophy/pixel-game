use crate::world::*;

// --- screen dimensions (static mut is safe: wasm is single-threaded) ---

static mut SCREEN_W: usize = 800;
static mut SCREEN_H: usize = 600;

#[inline(always)]
pub fn set_screen_size(w: usize, h: usize) {
    unsafe { SCREEN_W = w; SCREEN_H = h; }
}

#[inline(always)]
fn sw() -> usize { unsafe { SCREEN_W } }

#[inline(always)]
fn sh() -> usize { unsafe { SCREEN_H } }

// --- optimized drawing primitives ---

#[inline]
fn set_pixel(pixels: &mut [u8], x: i32, y: i32, r: u8, g: u8, b: u8) {
    let w = sw() as i32;
    let h = sh() as i32;
    if x >= 0 && x < w && y >= 0 && y < h {
        let idx = (y as usize * w as usize + x as usize) * 4;
        let p = &mut pixels[idx..idx + 4];
        p[0] = r; p[1] = g; p[2] = b; p[3] = 255;
    }
}

#[inline]
fn set_pixel_alpha(pixels: &mut [u8], x: i32, y: i32, r: u8, g: u8, b: u8, a: u8) {
    let w = sw() as i32;
    let h = sh() as i32;
    if x >= 0 && x < w && y >= 0 && y < h {
        let idx = (y as usize * w as usize + x as usize) * 4;
        let p = &mut pixels[idx..idx + 4];
        if a == 255 {
            p[0] = r; p[1] = g; p[2] = b; p[3] = 255;
        } else if a > 0 {
            let af = a as f32 * (1.0 / 255.0);
            let inv = 1.0 - af;
            p[0] = (r as f32 * af + p[0] as f32 * inv) as u8;
            p[1] = (g as f32 * af + p[1] as f32 * inv) as u8;
            p[2] = (b as f32 * af + p[2] as f32 * inv) as u8;
            p[3] = 255;
        }
    }
}

fn fill_rect(pixels: &mut [u8], x: i32, y: i32, w: i32, h: i32, r: u8, g: u8, b: u8) {
    let stride = sw();
    let x0 = x.max(0) as usize;
    let y0 = y.max(0) as usize;
    let x1 = (x + w).max(0).min(stride as i32) as usize;
    let y1 = (y + h).max(0).min(sh() as i32) as usize;
    if x0 >= x1 || y0 >= y1 { return; }
    let row_len = x1 - x0;
    // build a stamp for one row: [r,g,b,255, r,g,b,255, ...]
    let stamp: [u8; 4] = [r, g, b, 255];
    for py in y0..y1 {
        let row_start = (py * stride + x0) * 4;
        let row = &mut pixels[row_start..row_start + row_len * 4];
        for px_chunk in row.chunks_exact_mut(4) {
            px_chunk.copy_from_slice(&stamp);
        }
    }
}

fn fill_rect_alpha(pixels: &mut [u8], x: i32, y: i32, w: i32, h: i32, r: u8, g: u8, b: u8, a: u8) {
    let stride = sw();
    let x0 = x.max(0) as usize;
    let y0 = y.max(0) as usize;
    let x1 = (x + w).max(0).min(stride as i32) as usize;
    let y1 = (y + h).max(0).min(sh() as i32) as usize;
    if x0 >= x1 || y0 >= y1 { return; }
    let af = a as f32 * (1.0 / 255.0);
    let inv = 1.0 - af;
    let rf = r as f32 * af;
    let gf = g as f32 * af;
    let bf = b as f32 * af;
    for py in y0..y1 {
        let row_start = (py * stride + x0) * 4;
        let row = &mut pixels[row_start..(row_start + (x1 - x0) * 4)];
        for px_chunk in row.chunks_exact_mut(4) {
            px_chunk[0] = (rf + px_chunk[0] as f32 * inv) as u8;
            px_chunk[1] = (gf + px_chunk[1] as f32 * inv) as u8;
            px_chunk[2] = (bf + px_chunk[2] as f32 * inv) as u8;
            px_chunk[3] = 255;
        }
    }
}

pub fn clear(pixels: &mut [u8], r: u8, g: u8, b: u8) {
    let stamp: [u8; 4] = [r, g, b, 255];
    for chunk in pixels.chunks_exact_mut(4) {
        chunk.copy_from_slice(&stamp);
    }
}

pub fn render_tiles(pixels: &mut [u8], map: &WorldMap, cam: &Camera, time: f64) {
    let w = sw() as i32;
    let h = sh() as i32;
    let cx = cam.x as i32;
    let cy = cam.y as i32;
    let stx = (cx / TILE_SIZE as i32).max(0) as usize;
    let sty = (cy / TILE_SIZE as i32).max(0) as usize;
    let etx = ((cx + w) / TILE_SIZE as i32 + 1).min(MAP_W as i32) as usize;
    let ety = ((cy + h) / TILE_SIZE as i32 + 1).min(MAP_H as i32) as usize;

    for ty in sty..ety {
        for tx in stx..etx {
            let tile = map.tile_at(tx, ty);
            let (r, mut g, mut b) = tile.base_color();
            if tile == TileType::Water {
                let wave = ((time * 0.002 + tx as f64 * 0.5).sin() * 15.0) as i32;
                b = (b as i32 + wave).clamp(0, 255) as u8;
                g = (g as i32 + wave / 2).clamp(0, 255) as u8;
            }
            let sx = tx as i32 * TILE_SIZE as i32 - cx;
            let sy = ty as i32 * TILE_SIZE as i32 - cy;
            fill_rect(pixels, sx, sy, TILE_SIZE as i32, TILE_SIZE as i32, r, g, b);

            let er = r.saturating_sub(8);
            let eg = g.saturating_sub(8);
            let eb = b.saturating_sub(8);
            for i in 0..TILE_SIZE as i32 {
                set_pixel(pixels, sx + i, sy, er, eg, eb);
                set_pixel(pixels, sx, sy + i, er, eg, eb);
            }
        }
    }
}

fn draw_tree(pixels: &mut [u8], x: i32, y: i32) {
    fill_rect(pixels, x + 6, y + 8, 4, 8, 100, 70, 35);
    fill_rect(pixels, x + 2, y + 2, 12, 3, 20, 100, 25);
    fill_rect(pixels, x + 1, y - 2, 14, 5, 25, 115, 30);
    fill_rect(pixels, x + 3, y - 5, 10, 4, 30, 130, 35);
    fill_rect(pixels, x + 4, y - 3, 3, 2, 45, 150, 50);
    fill_rect(pixels, x + 8, y, 2, 2, 40, 140, 45);
}

fn draw_stump(pixels: &mut [u8], x: i32, y: i32) {
    fill_rect(pixels, x + 5, y + 10, 6, 4, 90, 60, 30);
    fill_rect(pixels, x + 6, y + 9, 4, 2, 110, 75, 40);
    set_pixel(pixels, x + 7, y + 10, 70, 50, 25);
    set_pixel(pixels, x + 8, y + 11, 70, 50, 25);
}

fn draw_rock(pixels: &mut [u8], x: i32, y: i32) {
    fill_rect(pixels, x + 3, y + 6, 10, 7, 130, 130, 135);
    fill_rect(pixels, x + 4, y + 4, 8, 3, 140, 140, 145);
    fill_rect(pixels, x + 5, y + 5, 3, 2, 160, 160, 168);
    fill_rect(pixels, x + 4, y + 12, 9, 2, 90, 90, 95);
}

fn draw_rock_rubble(pixels: &mut [u8], x: i32, y: i32) {
    fill_rect(pixels, x + 5, y + 11, 3, 2, 100, 100, 105);
    fill_rect(pixels, x + 9, y + 12, 2, 2, 90, 90, 95);
    set_pixel(pixels, x + 7, y + 12, 110, 110, 115);
}

pub fn render_objects_layer(
    pixels: &mut [u8], map: &WorldMap, cam: &Camera, hero_ty: usize, behind: bool,
) {
    let w = sw() as i32;
    let h = sh() as i32;
    let cx = cam.x as i32;
    let cy = cam.y as i32;
    for obj in &map.objects {
        let is_behind = obj.tile_y <= hero_ty;
        if is_behind != behind { continue; }
        let sx = obj.tile_x as i32 * TILE_SIZE as i32 - cx;
        let sy = obj.tile_y as i32 * TILE_SIZE as i32 - cy;
        if sx < -32 || sx > w + 16 || sy < -32 || sy > h + 16 { continue; }
        if obj.alive {
            match obj.kind {
                ObjectKind::Tree => draw_tree(pixels, sx, sy),
                ObjectKind::Rock => draw_rock(pixels, sx, sy),
            }
        } else {
            match obj.kind {
                ObjectKind::Tree => draw_stump(pixels, sx, sy),
                ObjectKind::Rock => draw_rock_rubble(pixels, sx, sy),
            }
        }
    }
}


pub fn render_chop_effect(pixels: &mut [u8], hero: &Hero, cam: &Camera, progress: f64) {
    let sx = (hero.world_x - cam.x) as i32;
    let sy = (hero.world_y - cam.y) as i32;
    let swing = ((progress * 8.0).sin() * 4.0) as i32;
    let ax: i32;
    let ay = sy + 6 + swing.abs();
    if hero.facing == 2 {
        ax = sx - 1;
        fill_rect(pixels, ax - 2, ay - 1, 3, 2, 160, 160, 170);
        fill_rect(pixels, ax, ay, 2, 4, 140, 100, 50);
    } else {
        ax = sx + 14;
        fill_rect(pixels, ax + 1, ay - 1, 3, 2, 160, 160, 170);
        fill_rect(pixels, ax, ay, 2, 4, 140, 100, 50);
    }
    let chip_phase = (progress * 12.0) as i32;
    for i in 0..3 {
        let offset = ((chip_phase + i * 37) % 7) as i32 - 3;
        let oy = ((chip_phase + i * 23) % 5) as i32 - 4;
        set_pixel(pixels, ax + offset, ay + oy, 140, 100, 40);
    }
}

pub fn render_target_marker(pixels: &mut [u8], hero: &Hero, cam: &Camera, time: f64) {
    if let Some(&(tx, ty)) = hero.path.last() {
        let sx = tx as i32 * TILE_SIZE as i32 - cam.x as i32;
        let sy = ty as i32 * TILE_SIZE as i32 - cam.y as i32;
        let pulse = ((time * 0.005).sin() * 3.0) as i32;
        let len = 4 + pulse;
        let s = TILE_SIZE as i32;
        let (r, g, b) = (255, 220, 100);
        for i in 0..len {
            set_pixel(pixels, sx + i, sy, r, g, b);
            set_pixel(pixels, sx, sy + i, r, g, b);
            set_pixel(pixels, sx + s - 1 - i, sy, r, g, b);
            set_pixel(pixels, sx + s - 1, sy + i, r, g, b);
            set_pixel(pixels, sx + i, sy + s - 1, r, g, b);
            set_pixel(pixels, sx, sy + s - 1 - i, r, g, b);
            set_pixel(pixels, sx + s - 1 - i, sy + s - 1, r, g, b);
            set_pixel(pixels, sx + s - 1, sy + s - 1 - i, r, g, b);
        }
    }
}

// ---- floating text ----

pub struct FloatingText {
    pub world_x: f64,
    pub world_y: f64,
    pub text: &'static str,
    pub timer: f64,
    pub duration: f64,
    pub color: (u8, u8, u8),
}

impl FloatingText {
    pub fn new(world_x: f64, world_y: f64, text: &'static str, color: (u8, u8, u8)) -> Self {
        FloatingText { world_x, world_y, text, timer: 0.0, duration: 1200.0, color }
    }
    pub fn update(&mut self, dt: f64) { self.timer += dt; self.world_y -= dt * 0.02; }
    pub fn alive(&self) -> bool { self.timer < self.duration }
}

pub fn render_floating_texts(pixels: &mut [u8], texts: &[FloatingText], cam: &Camera) {
    for ft in texts {
        if !ft.alive() { continue; }
        let alpha = ((1.0 - ft.timer / ft.duration) * 255.0) as u8;
        let sx = (ft.world_x - cam.x) as i32;
        let sy = (ft.world_y - cam.y) as i32;
        let (r, g, b) = ft.color;
        draw_tiny_string(pixels, sx, sy, ft.text, r, g, b, alpha);
    }
}

// ---- tiny 3x5 pixel font ----

const GLYPH_W: i32 = 4;

fn glyph(c: char) -> [u8; 5] {
    match c {
        'a' => [0b010, 0b101, 0b111, 0b101, 0b101],
        'b' => [0b110, 0b101, 0b110, 0b101, 0b110],
        'c' => [0b011, 0b100, 0b100, 0b100, 0b011],
        'd' => [0b110, 0b101, 0b101, 0b101, 0b110],
        'e' => [0b111, 0b100, 0b110, 0b100, 0b111],
        'f' => [0b111, 0b100, 0b110, 0b100, 0b100],
        'g' => [0b111, 0b100, 0b101, 0b101, 0b111],
        'h' => [0b101, 0b101, 0b111, 0b101, 0b101],
        'i' => [0b111, 0b010, 0b010, 0b010, 0b111],
        'j' => [0b001, 0b001, 0b001, 0b101, 0b010],
        'k' => [0b101, 0b110, 0b100, 0b110, 0b101],
        'l' => [0b100, 0b100, 0b100, 0b100, 0b111],
        'm' => [0b101, 0b111, 0b111, 0b101, 0b101],
        'n' => [0b101, 0b111, 0b111, 0b101, 0b101],
        'o' => [0b010, 0b101, 0b101, 0b101, 0b010],
        'p' => [0b110, 0b101, 0b110, 0b100, 0b100],
        'q' => [0b010, 0b101, 0b101, 0b110, 0b011],
        'r' => [0b110, 0b101, 0b110, 0b101, 0b101],
        's' => [0b011, 0b100, 0b010, 0b001, 0b110],
        't' => [0b111, 0b010, 0b010, 0b010, 0b010],
        'u' => [0b101, 0b101, 0b101, 0b101, 0b111],
        'v' => [0b101, 0b101, 0b101, 0b101, 0b010],
        'w' => [0b101, 0b101, 0b111, 0b111, 0b101],
        'x' => [0b101, 0b101, 0b010, 0b101, 0b101],
        'y' => [0b101, 0b101, 0b010, 0b010, 0b010],
        'z' => [0b111, 0b001, 0b010, 0b100, 0b111],
        '0' => [0b111, 0b101, 0b101, 0b101, 0b111],
        '1' => [0b010, 0b110, 0b010, 0b010, 0b111],
        '2' => [0b111, 0b001, 0b111, 0b100, 0b111],
        '3' => [0b111, 0b001, 0b111, 0b001, 0b111],
        '4' => [0b101, 0b101, 0b111, 0b001, 0b001],
        '5' => [0b111, 0b100, 0b111, 0b001, 0b111],
        '6' => [0b111, 0b100, 0b111, 0b101, 0b111],
        '7' => [0b111, 0b001, 0b010, 0b010, 0b010],
        '8' => [0b111, 0b101, 0b111, 0b101, 0b111],
        '9' => [0b111, 0b101, 0b111, 0b001, 0b111],
        '+' => [0b000, 0b010, 0b111, 0b010, 0b000],
        '-' => [0b000, 0b000, 0b111, 0b000, 0b000],
        '/' => [0b001, 0b001, 0b010, 0b100, 0b100],
        ':' => [0b000, 0b010, 0b000, 0b010, 0b000],
        ' ' => [0b000, 0b000, 0b000, 0b000, 0b000],
        _ =>   [0b111, 0b111, 0b111, 0b111, 0b111],
    }
}

fn draw_tiny_string(pixels: &mut [u8], x: i32, y: i32, text: &str, r: u8, g: u8, b: u8, alpha: u8) {
    let mut cx = x;
    for ch in text.chars() {
        let g_data = glyph(ch);
        for row in 0..5i32 {
            for col in 0..3i32 {
                if (g_data[row as usize] >> (2 - col)) & 1 == 1 {
                    set_pixel_alpha(pixels, cx + col, y + row, r, g, b, alpha);
                }
            }
        }
        cx += GLYPH_W;
    }
}

// ---- portrait (top-left, wow-style) ----

use crate::item::Inventory;
use crate::item::ItemId;
use crate::skills::WoodcuttingSkill;
use crate::HudState;

/// shared hero sprite drawing — used by both in-game and portrait
fn draw_hero_sprite(pixels: &mut [u8], sx: i32, sy: i32, facing: u32) {
    // shadow
    fill_rect(pixels, sx + 2, sy + 14, 12, 2, 15, 40, 15);
    // boots
    fill_rect(pixels, sx + 4, sy + 12, 3, 3, 60, 40, 25);
    fill_rect(pixels, sx + 9, sy + 12, 3, 3, 60, 40, 25);
    // legs
    fill_rect(pixels, sx + 4, sy + 10, 3, 3, 80, 65, 45);
    fill_rect(pixels, sx + 9, sy + 10, 3, 3, 80, 65, 45);
    // body
    fill_rect(pixels, sx + 3, sy + 5, 10, 6, 50, 100, 170);
    // belt
    fill_rect(pixels, sx + 3, sy + 9, 10, 1, 100, 70, 30);
    // arms
    fill_rect(pixels, sx + 1, sy + 5, 2, 5, 220, 180, 140);
    fill_rect(pixels, sx + 13, sy + 5, 2, 5, 220, 180, 140);
    // head
    fill_rect(pixels, sx + 4, sy + 1, 8, 5, 220, 180, 140);
    // hair
    fill_rect(pixels, sx + 3, sy, 10, 2, 80, 50, 20);
    // eyes
    if facing != 1 {
        set_pixel(pixels, sx + 6, sy + 3, 30, 30, 40);
        set_pixel(pixels, sx + 9, sy + 3, 30, 30, 40);
    }
}

pub fn render_hero(pixels: &mut [u8], hero: &Hero, cam: &Camera) {
    let sx = (hero.world_x - cam.x) as i32;
    let sy = (hero.world_y - cam.y) as i32;
    let bob = if !hero.path.is_empty() && hero.anim_frame == 1 { -1 } else { 0 };
    draw_hero_sprite(pixels, sx, sy + bob, hero.facing);
}

pub fn render_portrait(pixels: &mut [u8], hero: &Hero) {
    let px: i32 = 6;
    let py: i32 = 6;
    let frame_size: i32 = 22;

    // dark background
    fill_rect_alpha(pixels, px, py, frame_size, frame_size, 8, 8, 16, 220);

    // gold frame border
    let gold = (180, 150, 60);
    for i in 0..frame_size {
        set_pixel(pixels, px + i, py, gold.0, gold.1, gold.2);
        set_pixel(pixels, px + i, py + frame_size - 1, gold.0, gold.1, gold.2);
    }
    for i in 0..frame_size {
        set_pixel(pixels, px, py + i, gold.0, gold.1, gold.2);
        set_pixel(pixels, px + frame_size - 1, py + i, gold.0, gold.1, gold.2);
    }
    // inner highlight
    let gold2 = (200, 170, 80);
    for i in 1..frame_size - 1 {
        set_pixel(pixels, px + i, py + 1, gold2.0, gold2.1, gold2.2);
        set_pixel(pixels, px + 1, py + i, gold2.0, gold2.1, gold2.2);
    }

    // draw exact hero sprite centered in the frame
    let sprite_x = px + (frame_size - 16) / 2;
    let sprite_y = py + (frame_size - 16) / 2;
    draw_hero_sprite(pixels, sprite_x, sprite_y, 0);

    // --- health + mana bars TO THE RIGHT of portrait ---
    let bar_x = px + frame_size + 3;
    let bar_w: i32 = 70;
    let bar_h: i32 = 8;

    // health bar
    let hp_y = py + 1;
    fill_rect(pixels, bar_x, hp_y, bar_w, bar_h, 40, 15, 15);
    let hp_frac = if hero.max_health > 0 { hero.health as f64 / hero.max_health as f64 } else { 0.0 };
    let hp_w = (hp_frac * bar_w as f64) as i32;
    if hp_w > 0 {
        fill_rect(pixels, bar_x, hp_y, hp_w, bar_h, 180, 30, 30);
        fill_rect(pixels, bar_x, hp_y, hp_w, 2, 220, 60, 60);
    }
    // border
    for i in 0..bar_w {
        set_pixel(pixels, bar_x + i, hp_y, 60, 20, 20);
        set_pixel(pixels, bar_x + i, hp_y + bar_h - 1, 60, 20, 20);
    }
    let hp_text = format!("{}/{}", hero.health, hero.max_health);
    let ht_x = bar_x + (bar_w - hp_text.len() as i32 * GLYPH_W) / 2;
    draw_tiny_string(pixels, ht_x, hp_y + 2, &hp_text, 255, 255, 255, 220);

    // mana bar
    let mp_y = hp_y + bar_h + 2;
    fill_rect(pixels, bar_x, mp_y, bar_w, bar_h, 15, 15, 40);
    let mp_frac = if hero.max_mana > 0 { hero.mana as f64 / hero.max_mana as f64 } else { 0.0 };
    let mp_w = (mp_frac * bar_w as f64) as i32;
    if mp_w > 0 {
        fill_rect(pixels, bar_x, mp_y, mp_w, bar_h, 40, 60, 200);
        fill_rect(pixels, bar_x, mp_y, mp_w, 2, 70, 100, 240);
    }
    // border
    for i in 0..bar_w {
        set_pixel(pixels, bar_x + i, mp_y, 20, 20, 60);
        set_pixel(pixels, bar_x + i, mp_y + bar_h - 1, 20, 20, 60);
    }
    let mp_text = format!("{}/{}", hero.mana, hero.max_mana);
    let mt_x = bar_x + (bar_w - mp_text.len() as i32 * GLYPH_W) / 2;
    draw_tiny_string(pixels, mt_x, mp_y + 2, &mp_text, 255, 255, 255, 220);
}

// ---- hud buttons (bottom-right) ----

fn draw_button_frame(pixels: &mut [u8], x: i32, y: i32, size: i32, active: bool) {
    fill_rect_alpha(pixels, x, y, size, size, 12, 12, 20, 200);
    let border = if active { (180, 150, 60) } else { (60, 60, 80) };
    for i in 0..size {
        set_pixel(pixels, x + i, y, border.0, border.1, border.2);
        set_pixel(pixels, x + i, y + size - 1, border.0, border.1, border.2);
    }
    for i in 0..size {
        set_pixel(pixels, x, y + i, border.0, border.1, border.2);
        set_pixel(pixels, x + size - 1, y + i, border.0, border.1, border.2);
    }
    if active {
        for i in 1..size - 1 {
            set_pixel_alpha(pixels, x + i, y + 1, 200, 170, 80, 60);
            set_pixel_alpha(pixels, x + 1, y + i, 200, 170, 80, 40);
        }
    }
}

fn draw_bar_chart_icon(pixels: &mut [u8], x: i32, y: i32) {
    fill_rect(pixels, x + 3, y + 10, 4, 6, 60, 160, 60);
    fill_rect(pixels, x + 3, y + 10, 4, 1, 80, 200, 80);
    fill_rect(pixels, x + 8, y + 6, 4, 10, 60, 130, 180);
    fill_rect(pixels, x + 8, y + 6, 4, 1, 80, 160, 220);
    fill_rect(pixels, x + 13, y + 3, 4, 13, 180, 130, 50);
    fill_rect(pixels, x + 13, y + 3, 4, 1, 220, 160, 70);
    fill_rect(pixels, x + 2, y + 16, 17, 1, 100, 100, 110);
}

fn draw_bag_icon(pixels: &mut [u8], x: i32, y: i32) {
    fill_rect(pixels, x + 6, y + 2, 6, 2, 140, 100, 50);
    fill_rect(pixels, x + 5, y + 3, 2, 2, 140, 100, 50);
    fill_rect(pixels, x + 11, y + 3, 2, 2, 140, 100, 50);
    fill_rect(pixels, x + 4, y + 5, 10, 10, 120, 85, 40);
    fill_rect(pixels, x + 4, y + 5, 2, 10, 100, 70, 35);
    fill_rect(pixels, x + 12, y + 5, 2, 10, 100, 70, 35);
    fill_rect(pixels, x + 5, y + 14, 8, 1, 90, 60, 30);
    fill_rect(pixels, x + 6, y + 6, 3, 2, 150, 110, 55);
    fill_rect(pixels, x + 7, y + 9, 4, 3, 170, 150, 80);
    fill_rect(pixels, x + 8, y + 10, 2, 1, 120, 85, 40);
}

pub fn render_hud_buttons(pixels: &mut [u8], hud: &HudState) {
    let w = sw() as i32;
    let h = sh() as i32;
    let btn_size: i32 = 24;
    let btn_x = w - btn_size - 8;
    let inv_btn_y = h - btn_size - 8;
    let skills_btn_y = inv_btn_y - btn_size - 4;

    draw_button_frame(pixels, btn_x, skills_btn_y, btn_size, hud.skills_open);
    draw_bar_chart_icon(pixels, btn_x + 1, skills_btn_y + 1);

    draw_button_frame(pixels, btn_x, inv_btn_y, btn_size, hud.inventory_open);
    draw_bag_icon(pixels, btn_x + 2, inv_btn_y + 1);
}

// ---- collapsible panels ----

fn draw_panel_bg(pixels: &mut [u8], x: i32, y: i32, w: i32, h: i32) {
    fill_rect_alpha(pixels, x, y, w, h, 10, 10, 18, 210);
    for i in 0..w {
        set_pixel_alpha(pixels, x + i, y, 80, 75, 95, 160);
        set_pixel_alpha(pixels, x + i, y + h - 1, 55, 55, 70, 140);
    }
    for i in 0..h {
        set_pixel_alpha(pixels, x, y + i, 70, 70, 85, 140);
        set_pixel_alpha(pixels, x + w - 1, y + i, 55, 55, 70, 140);
    }
    for i in 1..w - 1 {
        set_pixel_alpha(pixels, x + i, y + 1, 100, 100, 120, 60);
    }
}

pub fn render_inventory_panel(pixels: &mut [u8], inventory: &Inventory) {
    let sw = sw() as i32;
    let sh = sh() as i32;
    let logs_count = inventory.count(ItemId::Logs);
    let stone_count = inventory.count(ItemId::Stone);

    let mut item_lines = 0;
    if logs_count > 0 { item_lines += 1; }
    if stone_count > 0 { item_lines += 1; }
    if item_lines == 0 { item_lines = 1; }

    let panel_w: i32 = 90;
    let panel_h: i32 = 14 + item_lines as i32 * 8;
    let btn_size: i32 = 24;
    let px = sw - panel_w - btn_size - 16;
    let py = sh - panel_h - 8;

    draw_panel_bg(pixels, px, py, panel_w, panel_h);

    let cx = px + 5;
    let mut cy = py + 4;

    draw_tiny_string(pixels, cx, cy, "inventory", 160, 155, 175, 255);
    cy += 9;

    let mut has_items = false;
    if logs_count > 0 {
        let text = format!("logs  x{}", logs_count);
        draw_tiny_string(pixels, cx, cy, &text, 200, 170, 80, 255);
        cy += 8;
        has_items = true;
    }
    if stone_count > 0 {
        let text = format!("stone x{}", stone_count);
        draw_tiny_string(pixels, cx, cy, &text, 160, 160, 180, 255);
        cy += 8;
        has_items = true;
    }
    if !has_items {
        draw_tiny_string(pixels, cx, cy, "empty", 70, 70, 85, 255);
    }
}

pub fn render_skills_panel(pixels: &mut [u8], wc: &WoodcuttingSkill) {
    let sw = sw() as i32;
    let sh = sh() as i32;
    let panel_w: i32 = 120;
    let panel_h: i32 = 30;
    let btn_size: i32 = 24;
    let px = sw - panel_w - btn_size - 16;
    let inv_btn_y = sh - btn_size - 8;
    let skills_btn_y = inv_btn_y - btn_size - 4;
    let py = skills_btn_y;

    draw_panel_bg(pixels, px, py, panel_w, panel_h);

    let cx = px + 5;
    let mut cy = py + 4;

    let title = format!("woodcutting lv {}", wc.level);
    draw_tiny_string(pixels, cx, cy, &title, 180, 180, 200, 255);
    cy += 8;

    let bar_w: i32 = panel_w - 12;
    let bar_h: i32 = 4;
    fill_rect(pixels, cx, cy, bar_w, bar_h, 30, 30, 40);
    let xp_base = WoodcuttingSkill::xp_for_level(wc.level);
    let xp_next = wc.xp_for_next_level();
    let xp_range = xp_next - xp_base;
    let xp_progress = if xp_range > 0 {
        ((wc.xp - xp_base) as f64 / xp_range as f64).min(1.0)
    } else { 0.0 };
    let fill_w = (xp_progress * bar_w as f64) as i32;
    if fill_w > 0 {
        fill_rect(pixels, cx, cy, fill_w, bar_h, 50, 160, 50);
        fill_rect(pixels, cx, cy, fill_w, 1, 70, 200, 70);
    }
    cy += bar_h + 2;

    let xp_text = format!("{}/{} xp", wc.xp, xp_next);
    draw_tiny_string(pixels, cx, cy, &xp_text, 100, 100, 120, 255);
}

// ---- minimap ----

pub fn render_minimap(pixels: &mut [u8], map: &WorldMap, hero: &Hero, cam: &Camera) {
    let sw = sw() as i32;
    let sh = sh() as i32;
    let mm = 80i32;
    let mx = sw - mm - 8;
    let my = 8;

    fill_rect(pixels, mx - 1, my - 1, mm + 2, mm + 2, 40, 40, 50);

    let sx = mm as f64 / MAP_W as f64;
    let sy = mm as f64 / MAP_H as f64;

    for ty in 0..MAP_H {
        for tx in 0..MAP_W {
            let (r, g, b) = map.tile_at(tx, ty).base_color();
            let px = mx + (tx as f64 * sx) as i32;
            let py = my + (ty as f64 * sy) as i32;
            set_pixel(pixels, px, py, r, g, b);
        }
    }

    let hx = mx + (hero.world_x / TILE_SIZE as f64 * sx) as i32;
    let hy = my + (hero.world_y / TILE_SIZE as f64 * sy) as i32;
    fill_rect(pixels, hx, hy, 2, 2, 255, 255, 100);

    let vx = mx + (cam.x / (MAP_W as f64 * TILE_SIZE as f64) * mm as f64) as i32;
    let vy = my + (cam.y / (MAP_H as f64 * TILE_SIZE as f64) * mm as f64) as i32;
    let vw = (sw as f64 / (MAP_W as f64 * TILE_SIZE as f64) * mm as f64) as i32;
    let vh = (sh as f64 / (MAP_H as f64 * TILE_SIZE as f64) * mm as f64) as i32;
    for i in 0..vw {
        set_pixel(pixels, vx + i, vy, 200, 200, 220);
        set_pixel(pixels, vx + i, vy + vh, 200, 200, 220);
    }
    for i in 0..vh {
        set_pixel(pixels, vx, vy + i, 200, 200, 220);
        set_pixel(pixels, vx + vw, vy + i, 200, 200, 220);
    }
}
