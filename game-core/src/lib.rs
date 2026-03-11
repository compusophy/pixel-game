pub mod world;
pub mod render;
pub mod pathfind;
pub mod item;
pub mod skills;
pub mod protocol;

use world::*;
use render::*;
use item::*;
use skills::*;
use protocol::{ServerMsg, PlayerSnapshot, ClientMsg};

#[derive(Debug, Clone, PartialEq)]
pub enum GameState {
    Login,
    Connecting,
    Playing,
}

pub struct PlayerState {
    pub current: PlayerSnapshot,
    pub target_x: f64,
    pub target_y: f64,
}

pub struct ClientState {
    pixels: Vec<u8>,
    width: usize,
    height: usize,
    
    pub map: WorldMap,
    pub camera: Camera,
    pub zoom: f64,

    pub my_player_id: u32,
    pub players: std::collections::HashMap<u32, PlayerState>,
    pub local_path: Vec<(usize, usize)>,
    pub floating_texts: Vec<FloatingText>,
    
    // UI state
    pub inventory: Inventory,
    pub woodcutting: WoodcuttingSkill,
    pub hud: HudState,
    pub pending_messages: Vec<ClientMsg>,

    pub state: GameState,
    pub username_input: String,
    pub connection_requested: Option<(String, bool)>, // (name, is_tutorial)
}

pub struct HudState {
    pub inventory_open: bool,
    pub skills_open: bool,
    pub map_open: bool,
    pub map_zoom: f64,
    pub map_cx: f64,
    pub map_cy: f64,
}

impl HudState {
    fn new() -> Self {
        HudState {
            inventory_open: false,
            skills_open: false,
            map_open: false,
            map_zoom: 1.0,
            map_cx: MAP_W as f64 / 2.0,
            map_cy: MAP_H as f64 / 2.0,
        }
    }
}



impl ClientState {
    pub fn new(w: usize, h: usize) -> Self {
        ClientState {
            pixels: vec![0; w * h * 4],
            width: w,
            height: h,
            map: WorldMap::generate(0), // Temp until Welcome
            camera: Camera::new(),
            zoom: 3.0,
            my_player_id: 0,
            players: std::collections::HashMap::new(),
            local_path: Vec::new(),
            floating_texts: Vec::new(),
            inventory: Inventory::new(),
            woodcutting: WoodcuttingSkill::new(),
            hud: HudState::new(),
            pending_messages: Vec::new(),
            state: GameState::Login,
            username_input: String::new(),
            connection_requested: None,
        }
    }

    pub fn receive_server_msg(&mut self, msg: ServerMsg) {
        match msg {
            ServerMsg::Welcome { player_id, map_seed, is_tutorial } => {
                self.my_player_id = player_id;
                if is_tutorial {
                    self.map = WorldMap::generate_tutorial();
                } else {
                    self.map = WorldMap::generate(map_seed);
                }
                self.state = GameState::Playing;
            }
            ServerMsg::Tick { players, .. } => {
                let mut new_ids = std::collections::HashSet::new();
                for p in players {
                    new_ids.insert(p.id);
                    let id = p.id;
                    if let Some(state) = self.players.get_mut(&id) {
                        state.target_x = p.world_x;
                        state.target_y = p.world_y;
                        
                        // If teleported
                        let dx = state.target_x - state.current.world_x;
                        let dy = state.target_y - state.current.world_y;
                        if dx*dx + dy*dy > 400.0 {
                            state.current.world_x = p.world_x;
                            state.current.world_y = p.world_y;
                        }
                        
                        state.current.is_moving = p.is_moving;
                        state.current.facing = p.facing;
                        state.current.anim_frame = p.anim_frame;
                        state.current.health = p.health;
                        state.current.max_health = p.max_health;
                        state.current.name = p.name.clone();
                    } else {
                        self.players.insert(id, PlayerState {
                            target_x: p.world_x,
                            target_y: p.world_y,
                            current: p,
                        });
                    }
                }
                self.players.retain(|k, _| new_ids.contains(k));

                // Clear local path correctly when we stop
                if let Some(me) = self.players.get(&self.my_player_id) {
                    if !me.current.is_moving && !self.local_path.is_empty() {
                        self.local_path.clear();
                    }
                }
            }
            ServerMsg::FloatingText { x, y, text, color } => {
                self.floating_texts.push(FloatingText::new(x, y, Box::leak(text.into_boxed_str()), color));
            }
        }
    }

    pub fn drain_messages(&mut self) -> Vec<ClientMsg> {
        std::mem::take(&mut self.pending_messages)
    }

    pub fn resize(&mut self, w: usize, h: usize) {
        self.width = w;
        self.height = h;
        self.pixels.resize(w * h * 4, 0);
    }

    pub fn set_zoom(&mut self, z: f64) {
        self.zoom = z.round().max(1.0).min(5.0);
        set_zoom(self.zoom);
    }

    pub fn on_scroll(&mut self, delta: f64, cursor_x: f64, cursor_y: f64) {
        if self.hud.map_open {
            let scr_w = self.width as f64;
            let scr_h = self.height as f64;
            let mm = scr_w.min(scr_h).min(300.0);
            let mx = (scr_w - mm) / 2.0;
            let my = (scr_h - mm) / 2.0;

            let vis_w = MAP_W as f64 / self.hud.map_zoom;
            let vis_h = MAP_H as f64 / self.hud.map_zoom;
            let left = (self.hud.map_cx - vis_w * 0.5).max(0.0).min(MAP_W as f64 - vis_w);
            let top = (self.hud.map_cy - vis_h * 0.5).max(0.0).min(MAP_H as f64 - vis_h);
            let cursor_tile_x = left + (cursor_x - mx) / mm * vis_w;
            let cursor_tile_y = top + (cursor_y - my) / mm * vis_h;

            let step = 0.3 * self.hud.map_zoom;
            if delta < 0.0 {
                self.hud.map_zoom = (self.hud.map_zoom + step).min(10.0);
            } else {
                self.hud.map_zoom = (self.hud.map_zoom - step).max(1.0);
            }

            let new_vis_w = MAP_W as f64 / self.hud.map_zoom;
            let new_vis_h = MAP_H as f64 / self.hud.map_zoom;
            self.hud.map_cx = cursor_tile_x - (cursor_x - mx) / mm * new_vis_w + new_vis_w / 2.0;
            self.hud.map_cy = cursor_tile_y - (cursor_y - my) / mm * new_vis_h + new_vis_h / 2.0;
            self.hud.map_cx = self.hud.map_cx.max(new_vis_w / 2.0).min(MAP_W as f64 - new_vis_w / 2.0);
            self.hud.map_cy = self.hud.map_cy.max(new_vis_h / 2.0).min(MAP_H as f64 - new_vis_h / 2.0);
        } else {
            let z_before = self.zoom;
            // Capture the world coordinate exactly under the cursor BEFORE zoom changes
            let world_x_before = cursor_x / z_before + self.camera.x;
            let world_y_before = cursor_y / z_before + self.camera.y;

            if delta < 0.0 {
                self.set_zoom(self.zoom + 1.0);
            } else {
                self.set_zoom(self.zoom - 1.0);
            }
            
            if self.zoom != z_before {
                // Adjust camera so the same world coordinate is still under the cursor AFTER zoom
                let new_camera_x = world_x_before - (cursor_x / self.zoom);
                let new_camera_y = world_y_before - (cursor_y / self.zoom);
                
                let vw = self.width as f64 / self.zoom;
                let vh = self.height as f64 / self.zoom;
                let mx = (MAP_W * TILE_SIZE) as f64 - vw;
                let my = (MAP_H * TILE_SIZE) as f64 - vh;
                
                self.camera.x = new_camera_x.max(0.0).min(mx);
                self.camera.y = new_camera_y.max(0.0).min(my);
            }
        }
    }

    pub fn pixels(&self) -> &[u8] {
        &self.pixels
    }
    pub fn width(&self) -> usize { self.width }
    pub fn height(&self) -> usize { self.height }

    pub fn tick(&mut self, dt: f64) {
        set_screen_size(self.width, self.height);
        set_zoom(self.zoom);

        if self.state == GameState::Login || self.state == GameState::Connecting {
            clear(&mut self.pixels, 10, 10, 18);
            render_login_screen(&mut self.pixels, &self.username_input, self.state == GameState::Connecting, dt);
            return;
        }

        // interpolate players
        for state in self.players.values_mut() {
            if state.current.world_x != state.target_x || state.current.world_y != state.target_y {
                let dx = state.target_x - state.current.world_x;
                let dy = state.target_y - state.current.world_y;
                let step = 0.08 * dt; // Match server speed
                let dist = (dx*dx + dy*dy).sqrt();
                if dist <= step {
                    state.current.world_x = state.target_x;
                    state.current.world_y = state.target_y;
                } else {
                    state.current.world_x += (dx / dist) * step;
                    state.current.world_y += (dy / dist) * step;
                }
            }
        }

        // find my player and sync camera
        if let Some(me) = self.players.get(&self.my_player_id) {
            let vw = self.width as f64 / self.zoom;
            let vh = self.height as f64 / self.zoom;
            // mock hero to pass to camera
            let mut pseudo_hero = Hero::new(0, 0);
            pseudo_hero.world_x = me.current.world_x;
            pseudo_hero.world_y = me.current.world_y;
            self.camera.follow(&pseudo_hero, vw, vh);
        }

        self.floating_texts.iter_mut().for_each(|ft| ft.update(dt));
        self.floating_texts.retain(|ft| ft.alive());

        clear(&mut self.pixels, 10, 10, 18);
        render_tiles(&mut self.pixels, &self.map, &self.camera, 0.0);
        
        // render objects
        // We will just assume my_ty is the screen center or my player
        let my_ty = if let Some(me) = self.players.get(&self.my_player_id) {
            (me.current.world_y / TILE_SIZE as f64) as usize
        } else { 0 };

        render_objects_layer(&mut self.pixels, &self.map, &self.camera, my_ty, true);
        
        // render all players
        for state in self.players.values() {
            let p = &state.current;
            let z = zm();
            let zi = z.round() as i32;
            let cam_px = (self.camera.x * z).floor() as i32;
            let cam_py = (self.camera.y * z).floor() as i32;
            let sx = (p.world_x * z).floor() as i32 - cam_px;
            let sy = (p.world_y * z).floor() as i32 - cam_py;
            
            draw_hero_sprite_z(&mut self.pixels, sx, sy, p.facing, zi, p.anim_frame, p.is_moving);

            // name tag
            let nw = p.name.len() as i32 * 4;
            draw_tiny_string(&mut self.pixels, sx + 8 * zi - nw / 2, sy - 6 * zi, &p.name, 255, 255, 255, 255);
        }

        render_objects_layer(&mut self.pixels, &self.map, &self.camera, my_ty, false);

        if !self.local_path.is_empty() {
            if let Some(me) = self.players.get(&self.my_player_id) {
                let mut pseudo_hero = Hero::new(0, 0);
                pseudo_hero.world_x = me.current.world_x;
                pseudo_hero.world_y = me.current.world_y;
                pseudo_hero.path = self.local_path.clone(); // Pass clone for rendering
                render_target_marker(&mut self.pixels, &pseudo_hero, &self.camera, dt);
            }
        }

        render_floating_texts(&mut self.pixels, &self.floating_texts, &self.camera);

        // UI rendering
        if let Some(me) = self.players.get(&self.my_player_id) {
            let mut pseudo_hero = Hero::new(0, 0);
            pseudo_hero.health = me.current.health as u32;
            pseudo_hero.max_health = me.current.max_health as u32;
            render_portrait(&mut self.pixels, &pseudo_hero);
        }
        
        render_hud_buttons(&mut self.pixels, &self.hud);
        if self.hud.inventory_open {
            render_inventory_panel(&mut self.pixels, &self.inventory);
        }
        if self.hud.skills_open {
            render_skills_panel(&mut self.pixels, &self.woodcutting);
        }
        if self.hud.map_open && !self.players.is_empty() {
            if let Some(me) = self.players.get(&self.my_player_id) {
                let mut pseudo_hero = Hero::new(0,0);
                pseudo_hero.world_x = me.current.world_x;
                pseudo_hero.world_y = me.current.world_y;
                render_minimap(
                    &mut self.pixels, &self.map, &pseudo_hero, &self.camera,
                    self.hud.map_zoom, self.hud.map_cx, self.hud.map_cy,
                );
            }
        }
    }

    pub fn on_click(&mut self, screen_x: f64, screen_y: f64) {
        if self.state == GameState::Login {
            let action = check_login_click(screen_x as i32, screen_y as i32, self.width as i32, self.height as i32);
            match action {
                LoginAction::Join => {
                    if !self.username_input.is_empty() {
                        self.connection_requested = Some((self.username_input.clone(), false));
                        self.state = GameState::Connecting;
                    }
                }
                LoginAction::Tutorial => {
                    let name = if self.username_input.is_empty() { "learner".to_string() } else { self.username_input.clone() };
                    self.connection_requested = Some((name, true));
                    self.state = GameState::Connecting;
                }
                LoginAction::None => {}
            }
            return;
        }

        let sx = screen_x as i32;
        let sy = screen_y as i32;

        let scr_w = self.width as i32;
        let scr_h = self.height as i32;
        let btn_size = 36;
        let p_start_x = scr_w - 3 * (btn_size + 4);
        let p_start_y = scr_h - btn_size - 4;

        if sx >= p_start_x && sx < p_start_x + btn_size && sy >= p_start_y && sy < p_start_y + btn_size {
            self.hud.inventory_open = !self.hud.inventory_open;
            if self.hud.inventory_open { self.hud.skills_open = false; }
            return;
        }
        if sx >= p_start_x + btn_size + 4 && sx < p_start_x + 2 * (btn_size + 4) && sy >= p_start_y && sy < p_start_y + btn_size {
            self.hud.skills_open = !self.hud.skills_open;
            if self.hud.skills_open { self.hud.inventory_open = false; }
            return;
        }

        let map_btn_x = scr_w - btn_size - 4;
        let map_btn_y = 8;
        if sx >= map_btn_x && sx < map_btn_x + btn_size && sy >= map_btn_y && sy < map_btn_y + btn_size {
            self.hud.map_open = !self.hud.map_open;
            if self.hud.map_open {
                self.hud.map_zoom = 1.0;
                self.hud.map_cx = MAP_W as f64 / 2.0;
                self.hud.map_cy = MAP_H as f64 / 2.0;
            }
            return;
        }

        if self.hud.map_open {
            self.hud.map_open = false;
            return; // don't move
        }

        // otherwise it's a world click, send to server
        let world_x = screen_x / self.zoom + self.camera.x;
        let world_y = screen_y / self.zoom + self.camera.y;
        self.pending_messages.push(ClientMsg::Click { world_x, world_y });

        // Calculate a local path instantly for the client-side visuals
        if let Some(me) = self.players.get(&self.my_player_id) {
            let hero_tx = (me.current.world_x / TILE_SIZE as f64) as usize;
            let hero_ty = (me.current.world_y / TILE_SIZE as f64) as usize;
            let cx = (world_x / TILE_SIZE as f64) as usize;
            let cy = (world_y / TILE_SIZE as f64) as usize;
            let tx = cx.min(MAP_W - 1);
            let ty = cy.min(MAP_H - 1);

            let mut goal = (tx, ty);
            let mut need_path = true;

            if !self.map.is_walkable(tx, ty) {
                if let Some(adj) = self.map.adjacent_walkable_tile(tx, ty, hero_tx, hero_ty) {
                    goal = adj;
                } else {
                    need_path = false;
                }
            }

            if need_path {
                let path = crate::pathfind::astar(&self.map, (hero_tx, hero_ty), goal);
                if !path.is_empty() {
                    self.local_path = path;
                }
            }
        }
    }

    pub fn on_key(&mut self, key: String, down: bool) {
        if !down { return; }
        if self.state != GameState::Login { return; }

        if key == "Backspace" {
            self.username_input.pop();
        } else if key == "Enter" {
            if !self.username_input.is_empty() {
                self.connection_requested = Some((self.username_input.clone(), false));
                self.state = GameState::Connecting;
            }
        } else if key.len() == 1 {
            let c = key.chars().next().unwrap();
            if self.username_input.len() < 12 && (c.is_alphanumeric() || c == ' ') {
                self.username_input.push(c.to_ascii_lowercase());
            }
        }
    }
}
