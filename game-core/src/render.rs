use crate::world::*;
use std::cell::Cell;

// --- screen dimensions + zoom (thread_local for multi-threaded safety) ---

thread_local! {
    static SCREEN_W: Cell<usize> = Cell::new(800);
    static SCREEN_H: Cell<usize> = Cell::new(600);
    static ZOOM: Cell<f64> = Cell::new(3.0);
}

#[inline(always)]
pub fn set_screen_size(w: usize, h: usize) {
    SCREEN_W.with(|c| c.set(w));
    SCREEN_H.with(|c| c.set(h));
}

#[inline(always)]
pub fn set_zoom(z: f64) { ZOOM.with(|c| c.set(z)); }

#[inline(always)]
fn sw() -> usize { SCREEN_W.with(|c| c.get()) }

#[inline(always)]
fn sh() -> usize { SCREEN_H.with(|c| c.get()) }

#[inline(always)]
fn zm() -> f64 { ZOOM.with(|c| c.get()) }

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
    let z = zm();
    let zi = z.round() as i32;
    let ts = TILE_SIZE as i32 * zi; // tile size in screen pixels (integer)
    // camera position in screen-pixel space (integer)
    let cam_px = (cam.x * z).floor() as i32;
    let cam_py = (cam.y * z).floor() as i32;
    let w = sw() as i32;
    let h = sh() as i32;
    // visible tile range
    let stx = (cam_px / ts).max(0) as usize;
    let sty = (cam_py / ts).max(0) as usize;
    let etx = (((cam_px + w) / ts) as usize + 2).min(MAP_W);
    let ety = (((cam_py + h) / ts) as usize + 2).min(MAP_H);

    for ty in sty..ety {
        for tx in stx..etx {
            let tile = map.tile_at(tx, ty);
            let (r, mut g, mut b) = tile.base_color();
            if tile == TileType::Water {
                let wave = ((time * 0.002 + tx as f64 * 0.5).sin() * 15.0) as i32;
                b = (b as i32 + wave).clamp(0, 255) as u8;
                g = (g as i32 + wave / 2).clamp(0, 255) as u8;
            }
            let sx = tx as i32 * ts - cam_px;
            let sy = ty as i32 * ts - cam_py;
            fill_rect(pixels, sx, sy, ts, ts, r, g, b);
        }
    }
}

// --- zoomed sprite helpers: all offsets/sizes multiplied by z ---

fn zr(pixels: &mut [u8], bx: i32, by: i32, z: i32, ox: i32, oy: i32, w: i32, h: i32, r: u8, g: u8, b: u8) {
    fill_rect(pixels, bx + ox * z, by + oy * z, w * z, h * z, r, g, b);
}
fn zp(pixels: &mut [u8], bx: i32, by: i32, z: i32, ox: i32, oy: i32, r: u8, g: u8, b: u8) {
    fill_rect(pixels, bx + ox * z, by + oy * z, z, z, r, g, b);
}

fn draw_tree(pixels: &mut [u8], x: i32, y: i32, z: i32) {
    zr(pixels, x, y, z, 6, 8, 4, 8, 100, 70, 35);
    zr(pixels, x, y, z, 2, 2, 12, 3, 20, 100, 25);
    zr(pixels, x, y, z, 1, -2, 14, 5, 25, 115, 30);
    zr(pixels, x, y, z, 3, -5, 10, 4, 30, 130, 35);
    zr(pixels, x, y, z, 4, -3, 3, 2, 45, 150, 50);
    zr(pixels, x, y, z, 8, 0, 2, 2, 40, 140, 45);
}

fn draw_stump(pixels: &mut [u8], x: i32, y: i32, z: i32) {
    zr(pixels, x, y, z, 5, 10, 6, 4, 90, 60, 30);
    zr(pixels, x, y, z, 6, 9, 4, 2, 110, 75, 40);
    zp(pixels, x, y, z, 7, 10, 70, 50, 25);
    zp(pixels, x, y, z, 8, 11, 70, 50, 25);
}

fn draw_rock(pixels: &mut [u8], x: i32, y: i32, z: i32) {
    zr(pixels, x, y, z, 3, 6, 10, 7, 130, 130, 135);
    zr(pixels, x, y, z, 4, 4, 8, 3, 140, 140, 145);
    zr(pixels, x, y, z, 5, 5, 3, 2, 160, 160, 168);
    zr(pixels, x, y, z, 4, 12, 9, 2, 90, 90, 95);
}

fn draw_rock_rubble(pixels: &mut [u8], x: i32, y: i32, z: i32) {
    zr(pixels, x, y, z, 5, 11, 3, 2, 100, 100, 105);
    zr(pixels, x, y, z, 9, 12, 2, 2, 90, 90, 95);
    zp(pixels, x, y, z, 7, 12, 110, 110, 115);
}

pub fn render_objects_layer(
    pixels: &mut [u8], map: &WorldMap, cam: &Camera, hero_ty: usize, behind: bool,
) {
    let z = zm();
    let zi = z.round() as i32;
    let ts = TILE_SIZE as i32 * zi;
    let cam_px = (cam.x * z).floor() as i32;
    let cam_py = (cam.y * z).floor() as i32;
    let w = sw() as i32;
    let h = sh() as i32;
    let margin = 32 * zi;
    for obj in &map.objects {
        let is_behind = obj.tile_y <= hero_ty;
        if is_behind != behind { continue; }
        let sx = obj.tile_x as i32 * ts - cam_px;
        let sy = obj.tile_y as i32 * ts - cam_py;
        if sx < -margin || sx > w + margin || sy < -margin || sy > h + margin { continue; }
        if obj.alive {
            match obj.kind {
                ObjectKind::Tree => draw_tree(pixels, sx, sy, zi),
                ObjectKind::Rock => draw_rock(pixels, sx, sy, zi),
            }
        } else {
            match obj.kind {
                ObjectKind::Tree => draw_stump(pixels, sx, sy, zi),
                ObjectKind::Rock => draw_rock_rubble(pixels, sx, sy, zi),
            }
        }
    }
}


pub fn render_chop_effect(pixels: &mut [u8], hero: &Hero, cam: &Camera, progress: f64) {
    let z = zm();
    let zi = z.round() as i32;
    let cam_px = (cam.x * z).floor() as i32;
    let cam_py = (cam.y * z).floor() as i32;
    let sx = (hero.world_x * z).floor() as i32 - cam_px;
    let sy = (hero.world_y * z).floor() as i32 - cam_py;
    let swing = ((progress * 8.0).sin() * 4.0) as i32 * zi;
    let ax: i32;
    let ay = sy + 6 * zi + swing.abs();
    if hero.facing == 2 {
        ax = sx - 1 * zi;
        fill_rect(pixels, ax - 2 * zi, ay - 1 * zi, 3 * zi, 2 * zi, 160, 160, 170);
        fill_rect(pixels, ax, ay, 2 * zi, 4 * zi, 140, 100, 50);
    } else {
        ax = sx + 14 * zi;
        fill_rect(pixels, ax + 1 * zi, ay - 1 * zi, 3 * zi, 2 * zi, 160, 160, 170);
        fill_rect(pixels, ax, ay, 2 * zi, 4 * zi, 140, 100, 50);
    }
    let chip_phase = (progress * 12.0) as i32;
    for i in 0..3 {
        let offset = (((chip_phase + i * 37) % 7) as i32 - 3) * zi;
        let oy = (((chip_phase + i * 23) % 5) as i32 - 4) * zi;
        fill_rect(pixels, ax + offset, ay + oy, zi, zi, 140, 100, 40);
    }
}

pub fn render_target_marker(pixels: &mut [u8], hero: &Hero, cam: &Camera, time: f64) {
    let z = zm();
    let zi = z.round() as i32;
    let ts = TILE_SIZE as i32 * zi;
    let cam_px = (cam.x * z).floor() as i32;
    let cam_py = (cam.y * z).floor() as i32;
    if let Some(&(tx, ty)) = hero.path.last() {
        let sx = tx as i32 * ts - cam_px;
        let sy = ty as i32 * ts - cam_py;
        let pulse = ((time * 0.005).sin() * 3.0) as i32;
        let len = (4 + pulse) * zi;
        let (r, g, b) = (255, 220, 100);
        for i in (0..len).step_by(zi as usize) {
            fill_rect(pixels, sx + i, sy, zi, zi, r, g, b);
            fill_rect(pixels, sx, sy + i, zi, zi, r, g, b);
            fill_rect(pixels, sx + ts - zi - i, sy, zi, zi, r, g, b);
            fill_rect(pixels, sx + ts - zi, sy + i, zi, zi, r, g, b);
            fill_rect(pixels, sx + i, sy + ts - zi, zi, zi, r, g, b);
            fill_rect(pixels, sx, sy + ts - zi - i, zi, zi, r, g, b);
            fill_rect(pixels, sx + ts - zi - i, sy + ts - zi, zi, zi, r, g, b);
            fill_rect(pixels, sx + ts - zi, sy + ts - zi - i, zi, zi, r, g, b);
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
    let z = zm();
    let cam_px = (cam.x * z).floor() as i32;
    let cam_py = (cam.y * z).floor() as i32;
    for ft in texts {
        if !ft.alive() { continue; }
        let alpha = ((1.0 - ft.timer / ft.duration) * 255.0) as u8;
        let sx = (ft.world_x * z).floor() as i32 - cam_px;
        let sy = (ft.world_y * z).floor() as i32 - cam_py;
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

/// shared hero sprite drawing at 1x — used by portrait (UI element)
fn draw_hero_sprite(pixels: &mut [u8], sx: i32, sy: i32, facing: u32) {
    draw_hero_sprite_z(pixels, sx, sy, facing, 1);
}

/// zoomed hero sprite — used by in-game rendering
fn draw_hero_sprite_z(pixels: &mut [u8], sx: i32, sy: i32, facing: u32, z: i32) {
    // shadow
    zr(pixels, sx, sy, z, 2, 14, 12, 2, 15, 40, 15);
    // boots
    zr(pixels, sx, sy, z, 4, 12, 3, 3, 60, 40, 25);
    zr(pixels, sx, sy, z, 9, 12, 3, 3, 60, 40, 25);
    // legs
    zr(pixels, sx, sy, z, 4, 10, 3, 3, 80, 65, 45);
    zr(pixels, sx, sy, z, 9, 10, 3, 3, 80, 65, 45);
    // body
    zr(pixels, sx, sy, z, 3, 5, 10, 6, 50, 100, 170);
    // belt
    zr(pixels, sx, sy, z, 3, 9, 10, 1, 100, 70, 30);
    // arms
    zr(pixels, sx, sy, z, 1, 5, 2, 5, 220, 180, 140);
    zr(pixels, sx, sy, z, 13, 5, 2, 5, 220, 180, 140);
    // head
    zr(pixels, sx, sy, z, 4, 1, 8, 5, 220, 180, 140);
    // hair
    zr(pixels, sx, sy, z, 3, 0, 10, 2, 80, 50, 20);
    // eyes
    if facing != 1 {
        zp(pixels, sx, sy, z, 6, 3, 30, 30, 40);
        zp(pixels, sx, sy, z, 9, 3, 30, 30, 40);
    }
}

pub fn render_hero(pixels: &mut [u8], hero: &Hero, cam: &Camera) {
    let z = zm();
    let zi = z.round() as i32;
    let cam_px = (cam.x * z).floor() as i32;
    let cam_py = (cam.y * z).floor() as i32;
    let sx = (hero.world_x * z).floor() as i32 - cam_px;
    let sy = (hero.world_y * z).floor() as i32 - cam_py;
    let bob = if !hero.path.is_empty() && hero.anim_frame == 1 { -zi } else { 0 };
    draw_hero_sprite_z(pixels, sx, sy + bob, hero.facing, zi);
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

    // --- LoL-style segmented health bar (10 segments) ---
    let segments = 10;
    let hp_y = py + 1;
    // dark bg
    fill_rect(pixels, bar_x, hp_y, bar_w, bar_h, 40, 15, 15);
    // filled portion
    let hp_frac = if hero.max_health > 0 { hero.health as f64 / hero.max_health as f64 } else { 0.0 };
    let hp_w = (hp_frac * bar_w as f64) as i32;
    if hp_w > 0 {
        fill_rect(pixels, bar_x, hp_y, hp_w, bar_h, 180, 30, 30);
        fill_rect(pixels, bar_x, hp_y, hp_w, 2, 220, 60, 60);
    }
    // segment tick marks
    for s in 1..segments {
        let tick_x = bar_x + (bar_w * s) / segments;
        for j in 0..bar_h {
            set_pixel(pixels, tick_x, hp_y + j, 60, 20, 20);
        }
    }
    // border
    for i in 0..bar_w {
        set_pixel(pixels, bar_x + i, hp_y, 60, 20, 20);
        set_pixel(pixels, bar_x + i, hp_y + bar_h - 1, 60, 20, 20);
    }

    // --- LoL-style segmented mana bar (10 segments) ---
    let mp_y = hp_y + bar_h + 2;
    fill_rect(pixels, bar_x, mp_y, bar_w, bar_h, 15, 15, 40);
    let mp_frac = if hero.max_mana > 0 { hero.mana as f64 / hero.max_mana as f64 } else { 0.0 };
    let mp_w = (mp_frac * bar_w as f64) as i32;
    if mp_w > 0 {
        fill_rect(pixels, bar_x, mp_y, mp_w, bar_h, 40, 60, 200);
        fill_rect(pixels, bar_x, mp_y, mp_w, 2, 70, 100, 240);
    }
    // segment tick marks
    for s in 1..segments {
        let tick_x = bar_x + (bar_w * s) / segments;
        for j in 0..bar_h {
            set_pixel(pixels, tick_x, mp_y + j, 20, 20, 60);
        }
    }
    // border
    for i in 0..bar_w {
        set_pixel(pixels, bar_x + i, mp_y, 20, 20, 60);
        set_pixel(pixels, bar_x + i, mp_y + bar_h - 1, 20, 20, 60);
    }
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

fn draw_map_icon(pixels: &mut [u8], x: i32, y: i32) {
    // parchment background
    fill_rect(pixels, x + 3, y + 3, 14, 14, 180, 160, 120);
    fill_rect(pixels, x + 3, y + 3, 14, 1, 200, 180, 140);
    // fold lines
    fill_rect(pixels, x + 7, y + 4, 1, 12, 150, 130, 100);
    fill_rect(pixels, x + 12, y + 4, 1, 12, 150, 130, 100);
    // x marker
    set_pixel(pixels, x + 9, y + 8, 180, 40, 40);
    set_pixel(pixels, x + 10, y + 9, 180, 40, 40);
    set_pixel(pixels, x + 11, y + 8, 180, 40, 40);
    set_pixel(pixels, x + 10, y + 7, 180, 40, 40);
}

pub fn render_hud_buttons(pixels: &mut [u8], hud: &HudState) {
    let w = sw() as i32;
    let h = sh() as i32;
    let btn_size: i32 = 24;

    // bottom-right: inventory + skills
    let btn_x = w - btn_size - 8;
    let inv_btn_y = h - btn_size - 8;
    let skills_btn_y = inv_btn_y - btn_size - 4;

    draw_button_frame(pixels, btn_x, skills_btn_y, btn_size, hud.skills_open);
    draw_bar_chart_icon(pixels, btn_x + 1, skills_btn_y + 1);

    draw_button_frame(pixels, btn_x, inv_btn_y, btn_size, hud.inventory_open);
    draw_bag_icon(pixels, btn_x + 2, inv_btn_y + 1);

    // top-right: map button
    let map_btn_x = w - btn_size - 8;
    let map_btn_y = 8;
    draw_button_frame(pixels, map_btn_x, map_btn_y, btn_size, hud.map_open);
    draw_map_icon(pixels, map_btn_x + 1, map_btn_y + 1);
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

// ---- minimap (centered overlay, toggled by map button) ----

pub fn render_minimap(
    pixels: &mut [u8], map: &WorldMap, hero: &Hero, cam: &Camera,
    map_zoom: f64, map_cx: f64, map_cy: f64,
) {
    let scr_w = sw() as i32;
    let scr_h = sh() as i32;
    let z = zm();

    // map viewport size on screen
    let mm = scr_w.min(scr_h).min(300);
    let mx = (scr_w - mm) / 2;
    let my = (scr_h - mm) / 2;

    // dark semi-transparent background
    fill_rect_alpha(pixels, mx - 4, my - 4, mm + 8, mm + 8, 10, 10, 18, 220);
    // border
    for i in 0..mm + 6 {
        set_pixel(pixels, mx - 3 + i, my - 3, 80, 75, 95);
        set_pixel(pixels, mx - 3 + i, my + mm + 2, 80, 75, 95);
    }
    for i in 0..mm + 6 {
        set_pixel(pixels, mx - 3, my - 3 + i, 80, 75, 95);
        set_pixel(pixels, mx + mm + 2, my - 3 + i, 80, 75, 95);
    }

    // how many tiles visible in the minimap at this zoom
    let vis_w = MAP_W as f64 / map_zoom;
    let vis_h = MAP_H as f64 / map_zoom;
    // top-left corner in tile coords
    let left = (map_cx - vis_w * 0.5).max(0.0).min(MAP_W as f64 - vis_w);
    let top = (map_cy - vis_h * 0.5).max(0.0).min(MAP_H as f64 - vis_h);

    // pixels per tile
    let ppt_x = mm as f64 / vis_w;
    let ppt_y = mm as f64 / vis_h;
    let ps_x = (ppt_x.ceil() as i32).max(1);
    let ps_y = (ppt_y.ceil() as i32).max(1);

    let stx = left.floor() as usize;
    let sty = top.floor() as usize;
    let etx = ((left + vis_w).ceil() as usize).min(MAP_W);
    let ety = ((top + vis_h).ceil() as usize).min(MAP_H);

    for ty in sty..ety {
        for tx in stx..etx {
            let (r, g, b) = map.tile_at(tx, ty).base_color();
            let px = mx + ((tx as f64 - left) * ppt_x) as i32;
            let py = my + ((ty as f64 - top) * ppt_y) as i32;
            if px >= mx && py >= my && px < mx + mm && py < my + mm {
                fill_rect(pixels, px, py, ps_x, ps_y, r, g, b);
            }
        }
    }

    // draw title
    let zoom_text = format!("world map · {:.0}×", map_zoom);
    draw_tiny_string(pixels, mx, my - 12, &zoom_text, 160, 155, 175, 255);

    // hero dot
    let hx = mx + ((hero.world_x / TILE_SIZE as f64 - left) * ppt_x) as i32;
    let hy = my + ((hero.world_y / TILE_SIZE as f64 - top) * ppt_y) as i32;
    if hx >= mx && hy >= my && hx < mx + mm && hy < my + mm {
        fill_rect(pixels, hx - 1, hy - 1, 4, 4, 255, 255, 100);
        fill_rect(pixels, hx, hy, 2, 2, 255, 200, 50);
    }

    // viewport rectangle
    let view_w = scr_w as f64 / z;
    let view_h = scr_h as f64 / z;
    let vx = mx + ((cam.x / TILE_SIZE as f64 - left) * ppt_x) as i32;
    let vy = my + ((cam.y / TILE_SIZE as f64 - top) * ppt_y) as i32;
    let vw = (view_w / TILE_SIZE as f64 * ppt_x) as i32;
    let vh = (view_h / TILE_SIZE as f64 * ppt_y) as i32;
    for i in 0..vw {
        set_pixel(pixels, vx + i, vy, 200, 200, 220);
        set_pixel(pixels, vx + i, vy + vh, 200, 200, 220);
    }
    for i in 0..vh {
        set_pixel(pixels, vx, vy + i, 200, 200, 220);
        set_pixel(pixels, vx + vw, vy + i, 200, 200, 220);
    }
}
