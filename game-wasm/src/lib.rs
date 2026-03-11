use wasm_bindgen::prelude::*;
use game_core::ClientState;
use game_core::protocol::{ServerMsg, ClientMsg};

#[wasm_bindgen]
pub struct PixelBuffer {
    inner: ClientState,
}

#[wasm_bindgen]
impl PixelBuffer {
    #[wasm_bindgen(constructor)]
    pub fn new(w: u32, h: u32) -> PixelBuffer {
        PixelBuffer {
            inner: ClientState::new(w as usize, h as usize),
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

    pub fn on_key(&mut self, key: String, down: bool) {
        self.inner.on_key(key, down);
    }

    pub fn poll_connection_request(&mut self) -> JsValue {
        if let Some((name, is_tutorial)) = self.inner.connection_requested.take() {
            // Return as a simple object { name, is_tutorial }
            let obj = js_sys::Object::new();
            js_sys::Reflect::set(&obj, &"name".into(), &name.into()).unwrap();
            js_sys::Reflect::set(&obj, &"is_tutorial".into(), &is_tutorial.into()).unwrap();
            obj.into()
        } else {
            JsValue::NULL
        }
    }

    // Process incoming binary message from server
    pub fn receive_message(&mut self, data: &[u8]) {
        if let Ok(msg) = bincode::deserialize::<ServerMsg>(data) {
            self.inner.receive_server_msg(msg);
        }
    }

    // Get any pending client messages to send to server
    // Returns a flattened array of bytes, prefixed by lengths if we needed multiple,
    // but for simplicity let's just return one at a time via a poll mechanism.
    pub fn poll_message(&mut self) -> Option<Vec<u8>> {
        let msgs = self.inner.drain_messages();
        if !msgs.is_empty() {
            // For now, if multiple queue up we drop some or we should queue them in the wrapper.
            // A better way: drain 1 at a time.
            let mut remaining = msgs;
            let first = remaining.remove(0);
            for m in remaining {
                self.inner.pending_messages.push(m);
            }
            return bincode::serialize(&first).ok();
        }
        None
    }

    // Create a Join message manually from JS
    pub fn create_join_msg(name: String, is_tutorial: bool) -> Vec<u8> {
        bincode::serialize(&ClientMsg::Join { name, is_tutorial }).unwrap_or_default()
    }
}
