pub mod world;
pub mod render;
pub mod pathfind;
pub mod item;
pub mod skills;

use world::*;
use render::*;
use item::*;
use skills::*;

#[derive(Clone)]
enum HeroAction {
    Idle,
    Chopping {
        target_idx: usize,
        timer: f64,
        chop_progress: f64,
    },
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

pub struct GameState {
    pixels: Vec<u8>,
    width: usize,
    height: usize,
    zoom: f64,
    map: WorldMap,
    hero: Hero,
    camera: Camera,
    last_time: f64,
    inventory: Inventory,
    woodcutting: WoodcuttingSkill,
    action: HeroAction,
    floating_texts: Vec<FloatingText>,
    hud: HudState,
}

impl GameState {
    pub fn new(w: usize, h: usize) -> GameState {
        let zoom = 3.0;
        set_screen_size(w, h);
        set_zoom(zoom);

        let map = WorldMap::generate(42);
        let hero = Hero::new(MAP_W / 2, MAP_H / 2);
        let mut camera = Camera::new();
        let vw = w as f64 / zoom;
        let vh = h as f64 / zoom;
        camera.snap_to(&hero, vw, vh);

        GameState {
            pixels: vec![0u8; w * h * 4],
            width: w,
            height: h,
            zoom,
            map,
            hero,
            camera,
            last_time: 0.0,
            inventory: Inventory::new(),
            woodcutting: WoodcuttingSkill::new(),
            action: HeroAction::Idle,
            floating_texts: Vec::new(),
            hud: HudState::new(),
        }
    }

    pub fn resize(&mut self, w: usize, h: usize) {
        self.width = w;
        self.height = h;
        self.pixels.resize(w * h * 4, 0);
        set_screen_size(w, h);
        let vw = w as f64 / self.zoom;
        let vh = h as f64 / self.zoom;
        self.camera.snap_to(&self.hero, vw, vh);
    }

    pub fn set_zoom(&mut self, z: f64) {
        self.zoom = z.round().max(1.0).min(5.0);
        set_zoom(self.zoom);
        let vw = self.width as f64 / self.zoom;
        let vh = self.height as f64 / self.zoom;
        self.camera.snap_to(&self.hero, vw, vh);
    }

    pub fn on_scroll(&mut self, delta: f64, cursor_x: f64, cursor_y: f64) {
        if self.hud.map_open {
            // zoom minimap centered on cursor position
            let scr_w = self.width as f64;
            let scr_h = self.height as f64;
            let mm = scr_w.min(scr_h).min(300.0);
            let mx = (scr_w - mm) / 2.0;
            let my = (scr_h - mm) / 2.0;

            // cursor position in map tile coords
            let vis_w = MAP_W as f64 / self.hud.map_zoom;
            let vis_h = MAP_H as f64 / self.hud.map_zoom;
            let left = (self.hud.map_cx - vis_w * 0.5).max(0.0).min(MAP_W as f64 - vis_w);
            let top = (self.hud.map_cy - vis_h * 0.5).max(0.0).min(MAP_H as f64 - vis_h);
            let cursor_tile_x = left + (cursor_x - mx) / mm * vis_w;
            let cursor_tile_y = top + (cursor_y - my) / mm * vis_h;

            let step = 0.3 * self.hud.map_zoom; // proportional step
            if delta < 0.0 {
                self.hud.map_zoom = (self.hud.map_zoom + step).min(10.0);
            } else {
                self.hud.map_zoom = (self.hud.map_zoom - step).max(1.0);
            }

            // adjust center so cursor stays over the same tile
            let new_vis_w = MAP_W as f64 / self.hud.map_zoom;
            let new_vis_h = MAP_H as f64 / self.hud.map_zoom;
            // cursor_tile = new_left + (cursor_x - mx) / mm * new_vis_w
            // new_left = new_cx - new_vis_w/2
            // => new_cx = cursor_tile - (cursor_x - mx)/mm * new_vis_w + new_vis_w/2
            self.hud.map_cx = cursor_tile_x - (cursor_x - mx) / mm * new_vis_w + new_vis_w / 2.0;
            self.hud.map_cy = cursor_tile_y - (cursor_y - my) / mm * new_vis_h + new_vis_h / 2.0;
            // clamp
            self.hud.map_cx = self.hud.map_cx.max(new_vis_w / 2.0).min(MAP_W as f64 - new_vis_w / 2.0);
            self.hud.map_cy = self.hud.map_cy.max(new_vis_h / 2.0).min(MAP_H as f64 - new_vis_h / 2.0);
        } else {
            // zoom game camera
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

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    pub fn tick(&mut self, time: f64) {
        set_screen_size(self.width, self.height);
        set_zoom(self.zoom);

        let dt = if self.last_time == 0.0 {
            16.0
        } else {
            (time - self.last_time).min(100.0)
        };
        self.last_time = time;

        self.hero.update(dt);
        let vw = self.width as f64 / self.zoom;
        let vh = self.height as f64 / self.zoom;
        self.camera.follow(&self.hero, vw, vh);

        let mut action = self.action.clone();
        match &mut action {
            HeroAction::Chopping { target_idx, timer, chop_progress } => {
                if self.hero.path.is_empty() {
                    *timer += dt;
                    *chop_progress += dt * 0.001;

                    if let Some(obj) = self.map.objects.get(*target_idx) {
                        let ox = obj.tile_x as f64 * TILE_SIZE as f64;
                        let oy = obj.tile_y as f64 * TILE_SIZE as f64;
                        let ddx = ox - self.hero.world_x;
                        let ddy = oy - self.hero.world_y;
                        if ddx.abs() > ddy.abs() {
                            self.hero.facing = if ddx > 0.0 { 3 } else { 2 };
                        } else {
                            self.hero.facing = if ddy > 0.0 { 0 } else { 1 };
                        }
                    }

                    if *timer >= 600.0 {
                        *timer -= 600.0;
                        let idx = *target_idx;
                        if let Some(obj) = self.map.objects.get_mut(idx) {
                            if obj.alive && obj.health > 0 {
                                obj.health -= 1;
                                if obj.health == 0 {
                                    obj.alive = false;
                                    self.inventory.add(ItemId::Logs, 1);
                                    self.woodcutting.add_xp(25);
                                    let fx = obj.tile_x as f64 * TILE_SIZE as f64;
                                    let fy = obj.tile_y as f64 * TILE_SIZE as f64 - 8.0;
                                    self.floating_texts.push(FloatingText::new(
                                        fx, fy, "+1 logs", (220, 180, 80),
                                    ));
                                    self.floating_texts.push(FloatingText::new(
                                        fx, fy - 10.0, "+25 xp", (100, 220, 100),
                                    ));
                                    action = HeroAction::Idle;
                                    self.action = action;
                                    self.render_frame(time, dt);
                                    return;
                                }
                            } else {
                                action = HeroAction::Idle;
                                self.action = action;
                                self.render_frame(time, dt);
                                return;
                            }
                        }
                    }
                }
            }
            HeroAction::Idle => {}
        }
        self.action = action;
        self.render_frame(time, dt);
    }

    fn render_frame(&mut self, time: f64, dt: f64) {
        self.floating_texts.iter_mut().for_each(|ft| ft.update(dt));
        self.floating_texts.retain(|ft| ft.alive());

        clear(&mut self.pixels, 10, 10, 18);
        render_tiles(&mut self.pixels, &self.map, &self.camera, time);
        render_target_marker(&mut self.pixels, &self.hero, &self.camera, time);
        render_objects_layer(&mut self.pixels, &self.map, &self.camera, self.hero.tile_y(), true);
        render_hero(&mut self.pixels, &self.hero, &self.camera);

        if let HeroAction::Chopping { chop_progress, .. } = &self.action {
            if self.hero.path.is_empty() {
                render_chop_effect(&mut self.pixels, &self.hero, &self.camera, *chop_progress);
            }
        }

        render_objects_layer(&mut self.pixels, &self.map, &self.camera, self.hero.tile_y(), false);
        render_floating_texts(&mut self.pixels, &self.floating_texts, &self.camera);

        // ui layer (not affected by zoom)
        render_portrait(&mut self.pixels, &self.hero);
        render_hud_buttons(&mut self.pixels, &self.hud);
        if self.hud.inventory_open {
            render_inventory_panel(&mut self.pixels, &self.inventory);
        }
        if self.hud.skills_open {
            render_skills_panel(&mut self.pixels, &self.woodcutting);
        }
        if self.hud.map_open {
            render_minimap(
                &mut self.pixels, &self.map, &self.hero, &self.camera,
                self.hud.map_zoom, self.hud.map_cx, self.hud.map_cy,
            );
        }
    }

    pub fn on_click(&mut self, screen_x: f64, screen_y: f64) {
        let w = self.width as i32;
        let h = self.height as i32;
        let sx = screen_x as i32;
        let sy = screen_y as i32;

        let btn_size: i32 = 24;

        // bottom-right buttons
        let btn_x = w - btn_size - 8;
        let inv_btn_y = h - btn_size - 8;
        let skills_btn_y = inv_btn_y - btn_size - 4;

        if sx >= btn_x && sx < btn_x + btn_size && sy >= inv_btn_y && sy < inv_btn_y + btn_size {
            self.hud.inventory_open = !self.hud.inventory_open;
            return;
        }
        if sx >= btn_x && sx < btn_x + btn_size && sy >= skills_btn_y && sy < skills_btn_y + btn_size {
            self.hud.skills_open = !self.hud.skills_open;
            return;
        }

        // top-right map button
        let map_btn_x = w - btn_size - 8;
        let map_btn_y = 8;
        if sx >= map_btn_x && sx < map_btn_x + btn_size && sy >= map_btn_y && sy < map_btn_y + btn_size {
            self.hud.map_open = !self.hud.map_open;
            if self.hud.map_open {
                // reset map view
                self.hud.map_zoom = 1.0;
                self.hud.map_cx = MAP_W as f64 / 2.0;
                self.hud.map_cy = MAP_H as f64 / 2.0;
            }
            return;
        }

        // if map overlay is open, clicking anywhere else closes it
        if self.hud.map_open {
            self.hud.map_open = false;
            return;
        }

        if self.hud.inventory_open {
            let panel_w: i32 = 110;
            let panel_h: i32 = 80;
            let px = w - panel_w - btn_size - 16;
            let py = h - panel_h - 8;
            if sx >= px && sx < px + panel_w && sy >= py && sy < py + panel_h {
                return;
            }
        }
        if self.hud.skills_open {
            let panel_w: i32 = 120;
            let panel_h: i32 = 50;
            let px = w - panel_w - btn_size - 16;
            let py = h - btn_size - 4 - 24 - panel_h + 16;
            if sx >= px && sx < px + panel_w && sy >= py && sy < py + panel_h {
                return;
            }
        }

        if sx < 52 && sy < 52 {
            return;
        }

        // convert screen coords to world coords using zoom
        let world_x = screen_x / self.zoom + self.camera.x;
        let world_y = screen_y / self.zoom + self.camera.y;
        let tile_x = (world_x / TILE_SIZE as f64) as usize;
        let tile_y = (world_y / TILE_SIZE as f64) as usize;

        if let Some(obj_idx) = self.map.object_index_at(tile_x, tile_y) {
            let obj = &self.map.objects[obj_idx];
            if obj.kind == ObjectKind::Tree && obj.alive {
                if let Some(adj) = self.map.adjacent_walkable_tile(
                    tile_x, tile_y,
                    self.hero.tile_x(), self.hero.tile_y(),
                ) {
                    let start = (self.hero.tile_x(), self.hero.tile_y());
                    let path = pathfind::astar(&self.map, start, adj);
                    if !path.is_empty() || start == adj {
                        self.hero.path = path;
                        self.action = HeroAction::Chopping {
                            target_idx: obj_idx,
                            timer: 0.0,
                            chop_progress: 0.0,
                        };
                        return;
                    }
                }
            }
        }

        self.action = HeroAction::Idle;
        let start = (self.hero.tile_x(), self.hero.tile_y());
        let path = pathfind::astar(&self.map, start, (tile_x, tile_y));
        if !path.is_empty() {
            self.hero.path = path;
        }
    }
}
