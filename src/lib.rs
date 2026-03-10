use wasm_bindgen::prelude::*;

mod world;
mod render;
mod pathfind;
mod item;
mod skills;

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
}

impl HudState {
    fn new() -> Self {
        HudState {
            inventory_open: false,
            skills_open: false,
        }
    }
}

#[wasm_bindgen]
pub struct PixelBuffer {
    pixels: Vec<u8>,
    width: usize,
    height: usize,
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

#[wasm_bindgen]
impl PixelBuffer {
    #[wasm_bindgen(constructor)]
    pub fn new(w: u32, h: u32) -> PixelBuffer {
        let width = w as usize;
        let height = h as usize;
        set_screen_size(width, height);

        let map = WorldMap::generate(42);
        let hero = Hero::new(MAP_W / 2, MAP_H / 2);
        let mut camera = Camera::new();
        camera.snap_to(&hero, width as f64, height as f64);

        PixelBuffer {
            pixels: vec![0u8; width * height * 4],
            width,
            height,
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

    pub fn resize(&mut self, w: u32, h: u32) {
        let width = w as usize;
        let height = h as usize;
        self.width = width;
        self.height = height;
        self.pixels.resize(width * height * 4, 0);
        set_screen_size(width, height);
        self.camera.snap_to(&self.hero, width as f64, height as f64);
    }

    pub fn pointer(&self) -> *const u8 {
        self.pixels.as_ptr()
    }

    pub fn width(&self) -> u32 {
        self.width as u32
    }

    pub fn height(&self) -> u32 {
        self.height as u32
    }

    pub fn tick(&mut self, time: f64) {
        // ensure screen size is set for this frame
        set_screen_size(self.width, self.height);

        let dt = if self.last_time == 0.0 {
            16.0
        } else {
            (time - self.last_time).min(100.0)
        };
        self.last_time = time;

        // update hero movement
        self.hero.update(dt);
        self.camera.follow(&self.hero, self.width as f64, self.height as f64);

        // chopping logic
        let mut action = self.action.clone();
        match &mut action {
            HeroAction::Chopping { target_idx, timer, chop_progress } => {
                // only chop when hero has arrived (no more path)
                if self.hero.path.is_empty() {
                    *timer += dt;
                    *chop_progress += dt * 0.001;

                    // face the tree
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

                    // every 600ms, one chop hit
                    if *timer >= 600.0 {
                        *timer -= 600.0;
                        let idx = *target_idx;
                        if let Some(obj) = self.map.objects.get_mut(idx) {
                            if obj.alive && obj.health > 0 {
                                obj.health -= 1;
                                if obj.health == 0 {
                                    obj.alive = false;
                                    // grant items + xp
                                    let xp_amount = 25;
                                    self.inventory.add(ItemId::Logs, 1);
                                    self.woodcutting.add_xp(xp_amount);
                                    // spawn floating texts
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
        // update floating texts
        self.floating_texts.iter_mut().for_each(|ft| ft.update(dt));
        self.floating_texts.retain(|ft| ft.alive());

        clear(&mut self.pixels, 10, 10, 18);
        render_tiles(&mut self.pixels, &self.map, &self.camera, time);
        render_target_marker(&mut self.pixels, &self.hero, &self.camera, time);
        render_objects_layer(&mut self.pixels, &self.map, &self.camera, self.hero.tile_y(), true);
        render_hero(&mut self.pixels, &self.hero, &self.camera);

        // render chop effect if chopping
        if let HeroAction::Chopping { chop_progress, .. } = &self.action {
            if self.hero.path.is_empty() {
                render_chop_effect(&mut self.pixels, &self.hero, &self.camera, *chop_progress);
            }
        }

        render_objects_layer(&mut self.pixels, &self.map, &self.camera, self.hero.tile_y(), false);
        render_floating_texts(&mut self.pixels, &self.floating_texts, &self.camera);
        render_minimap(&mut self.pixels, &self.map, &self.hero, &self.camera);

        // hud: portrait + bars top-left, buttons + panels bottom-right
        render_portrait(&mut self.pixels, &self.hero);
        render_hud_buttons(&mut self.pixels, &self.hud);
        if self.hud.inventory_open {
            render_inventory_panel(&mut self.pixels, &self.inventory);
        }
        if self.hud.skills_open {
            render_skills_panel(&mut self.pixels, &self.woodcutting);
        }
    }

    pub fn on_click(&mut self, screen_x: f64, screen_y: f64) {
        let w = self.width as i32;
        let h = self.height as i32;
        let sx = screen_x as i32;
        let sy = screen_y as i32;

        // check hud button clicks first
        // button layout: bottom-right, two 24x24 squares stacked vertically
        let btn_size: i32 = 24;
        let btn_x = w - btn_size - 8;
        let inv_btn_y = h - btn_size - 8;
        let skills_btn_y = inv_btn_y - btn_size - 4;

        // inventory button
        if sx >= btn_x && sx < btn_x + btn_size && sy >= inv_btn_y && sy < inv_btn_y + btn_size {
            self.hud.inventory_open = !self.hud.inventory_open;
            return;
        }
        // skills button
        if sx >= btn_x && sx < btn_x + btn_size && sy >= skills_btn_y && sy < skills_btn_y + btn_size {
            self.hud.skills_open = !self.hud.skills_open;
            return;
        }

        // check if click is inside an open panel (absorb it, don't walk)
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

        // check portrait area (don't walk when clicking portrait)
        if sx < 52 && sy < 52 {
            return;
        }

        // world click logic
        let world_x = screen_x + self.camera.x;
        let world_y = screen_y + self.camera.y;
        let tile_x = (world_x / TILE_SIZE as f64) as usize;
        let tile_y = (world_y / TILE_SIZE as f64) as usize;

        // check if we clicked on a tree/resource
        if let Some(obj_idx) = self.map.object_index_at(tile_x, tile_y) {
            let obj = &self.map.objects[obj_idx];
            if obj.kind == ObjectKind::Tree && obj.alive {
                // find adjacent tile to stand on
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

        // normal movement
        self.action = HeroAction::Idle;
        let start = (self.hero.tile_x(), self.hero.tile_y());
        let path = pathfind::astar(&self.map, start, (tile_x, tile_y));
        if !path.is_empty() {
            self.hero.path = path;
        }
    }
}
