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
use protocol::{ServerMsg, PlayerSnapshot, ObjectUpdate, ClientMsg};

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

pub struct ClientState {
    pixels: Vec<u8>,
    width: usize,
    height: usize,
    
    pub map: WorldMap,
    pub camera: Camera,
    pub zoom: f64,

    pub my_player_id: u32,
    pub players: Vec<PlayerSnapshot>,
    pub floating_texts: Vec<FloatingText>,
    
    // UI state
    pub inventory: Inventory,
    pub woodcutting: WoodcuttingSkill,
    pub hud: HudState,
    pub pending_messages: Vec<ClientMsg>,
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
            players: Vec::new(),
            floating_texts: Vec::new(),
            inventory: Inventory::new(),
            woodcutting: WoodcuttingSkill::new(),
            hud: HudState::new(),
            pending_messages: Vec::new(),
        }
    }

    pub fn receive_server_msg(&mut self, msg: ServerMsg) {
        match msg {
            ServerMsg::Welcome { player_id, map_seed } => {
                self.my_player_id = player_id;
                self.map = WorldMap::generate(map_seed);
            }
            ServerMsg::Tick { players, objects } => {
                self.players = players;
                // TODO: Update objects when we send dirty states
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
            if delta < 0.0 {
                self.set_zoom(self.zoom + 1.0);
            } else {
                self.set_zoom(self.zoom - 1.0);
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

        // find my player and sync camera
        if let Some(me) = self.players.iter().find(|p| p.id == self.my_player_id) {
            let vw = self.width as f64 / self.zoom;
            let vh = self.height as f64 / self.zoom;
            // mock hero to pass to camera
            let mut pseudo_hero = Hero::new(0, 0);
            pseudo_hero.world_x = me.world_x;
            pseudo_hero.world_y = me.world_y;
            self.camera.follow(&pseudo_hero, vw, vh);
        }

        self.floating_texts.iter_mut().for_each(|ft| ft.update(dt));
        self.floating_texts.retain(|ft| ft.alive());

        clear(&mut self.pixels, 10, 10, 18);
        render_tiles(&mut self.pixels, &self.map, &self.camera, 0.0);
        
        // render objects
        // We will just assume my_ty is the screen center or my player
        let my_ty = if let Some(me) = self.players.iter().find(|p| p.id == self.my_player_id) {
            (me.world_y / TILE_SIZE as f64) as usize
        } else { 0 };

        render_objects_layer(&mut self.pixels, &self.map, &self.camera, my_ty, true);
        
        // render all players
        for p in &self.players {
            let z = zm();
            let zi = z.round() as i32;
            let cam_px = (self.camera.x * z).floor() as i32;
            let cam_py = (self.camera.y * z).floor() as i32;
            let sx = (p.world_x * z).floor() as i32 - cam_px;
            let sy = (p.world_y * z).floor() as i32 - cam_py;
            let bob = if p.is_moving && p.anim_frame == 1 { -zi } else { 0 };
            draw_hero_sprite_z(&mut self.pixels, sx, sy + bob, p.facing, zi);

            // name tag
            let nw = p.name.len() as i32 * 4;
            draw_tiny_string(&mut self.pixels, sx + 8 * zi - nw / 2, sy - 6 * zi, &p.name, 255, 255, 255, 255);
        }

        render_objects_layer(&mut self.pixels, &self.map, &self.camera, my_ty, false);
        render_floating_texts(&mut self.pixels, &self.floating_texts, &self.camera);

        // UI rendering
        if let Some(me) = self.players.iter().find(|p| p.id == self.my_player_id) {
            let mut pseudo_hero = Hero::new(0, 0);
            pseudo_hero.health = me.health as u32;
            pseudo_hero.max_health = me.max_health as u32;
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
            if let Some(me) = self.players.iter().find(|p| p.id == self.my_player_id) {
                let mut pseudo_hero = Hero::new(0,0);
                pseudo_hero.world_x = me.world_x;
                pseudo_hero.world_y = me.world_y;
                render_minimap(
                    &mut self.pixels, &self.map, &pseudo_hero, &self.camera,
                    self.hud.map_zoom, self.hud.map_cx, self.hud.map_cy,
                );
            }
        }
    }

    pub fn on_click(&mut self, screen_x: f64, screen_y: f64) {
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
    }
}
