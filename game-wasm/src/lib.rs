use wasm_bindgen::prelude::*;
use game_core::GameState;

#[wasm_bindgen]
pub struct PixelBuffer {
    inner: GameState,
}

#[wasm_bindgen]
impl PixelBuffer {
    #[wasm_bindgen(constructor)]
    pub fn new(w: u32, h: u32) -> PixelBuffer {
        PixelBuffer {
            inner: GameState::new(w as usize, h as usize),
        }
    }

    pub fn resize(&mut self, w: u32, h: u32) {
        self.inner.resize(w as usize, h as usize);
    }

    pub fn pointer(&self) -> *const u8 {
        self.inner.pixels().as_ptr()
    }

    pub fn width(&self) -> u32 {
        self.inner.width() as u32
    }

    pub fn height(&self) -> u32 {
        self.inner.height() as u32
    }

    pub fn tick(&mut self, time: f64) {
        self.inner.tick(time);
    }

    pub fn on_click(&mut self, screen_x: f64, screen_y: f64) {
        self.inner.on_click(screen_x, screen_y);
    }

    pub fn set_zoom(&mut self, z: f64) {
        self.inner.set_zoom(z);
    }

    pub fn on_scroll(&mut self, delta: f64, cursor_x: f64, cursor_y: f64) {
        self.inner.on_scroll(delta, cursor_x, cursor_y);
    }
}
