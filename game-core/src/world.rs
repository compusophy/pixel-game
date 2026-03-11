use serde::{Serialize, Deserialize};

pub const TILE_SIZE: usize = 16;
pub const MAP_W: usize = 200;
pub const MAP_H: usize = 200;

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize, Debug)]
pub enum TileType {
    Grass,
    GrassDark,
    Dirt,
    Sand,
    Water,
    Stone,
}

impl TileType {
    pub fn is_walkable(self) -> bool {
        !matches!(self, TileType::Water)
    }

    pub fn base_color(self) -> (u8, u8, u8) {
        match self {
            TileType::Grass => (34, 120, 34),
            TileType::GrassDark => (28, 100, 28),
            TileType::Dirt => (140, 105, 60),
            TileType::Sand => (194, 178, 128),
            TileType::Water => (30, 80, 160),
            TileType::Stone => (120, 120, 125),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize, Debug)]
pub enum ObjectKind {
    Tree,
    Rock,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct WorldObject {
    pub kind: ObjectKind,
    pub tile_x: usize,
    pub tile_y: usize,
    pub health: u32,
    pub alive: bool,
}

pub struct WorldMap {
    pub tiles: Vec<Vec<TileType>>,
    pub objects: Vec<WorldObject>,
}

fn hash(x: u32, y: u32, seed: u32) -> u32 {
    let mut h = seed
        .wrapping_add(x.wrapping_mul(374761393))
        .wrapping_add(y.wrapping_mul(668265263));
    h = (h ^ (h >> 13)).wrapping_mul(1274126177);
    h = (h ^ (h >> 16)).wrapping_mul(2654435761);
    h ^ (h >> 13)
}

fn hash_f(x: u32, y: u32, seed: u32) -> f64 {
    (hash(x, y, seed) & 0xFFFF) as f64 / 65535.0
}

impl WorldMap {
    pub fn generate(seed: u32) -> Self {
        // we just use Grass everywhere now per user request
        let mut tiles = vec![vec![TileType::Grass; MAP_W]; MAP_H];

        // water lakes
        let lakes = [(15u32, 12u32, 5u32), (45, 40, 4), (10, 50, 3), (50, 15, 4)];
        for &(cx, cy, radius) in &lakes {
            for dy in 0..MAP_H {
                for dx in 0..MAP_W {
                    let dist_sq = ((dx as i32 - cx as i32).pow(2)
                        + (dy as i32 - cy as i32).pow(2))
                        as u32;
                    if dist_sq <= radius * radius {
                        tiles[dy][dx] = TileType::Water;
                    } else if dist_sq <= (radius + 1) * (radius + 1)
                        && tiles[dy][dx] != TileType::Water
                    {
                        tiles[dy][dx] = TileType::Sand;
                    }
                }
            }
        }

        // dirt paths
        for x in 5..55 {
            tiles[32][x] = TileType::Dirt;
            tiles[33][x] = TileType::Dirt;
        }
        for y in 10..55 {
            tiles[y][32] = TileType::Dirt;
            tiles[y][33] = TileType::Dirt;
        }

        // stone patches
        for &(cx, cy) in &[(25usize, 25usize), (48, 30)] {
            for dy in 0..3 {
                for dx in 0..4 {
                    let tx = (cx + dx).min(MAP_W - 1);
                    let ty = (cy + dy).min(MAP_H - 1);
                    if tiles[ty][tx] != TileType::Water {
                        tiles[ty][tx] = TileType::Stone;
                    }
                }
            }
        }

        // place trees and rocks
        let mut objects = Vec::new();
        // start at y=2 to ensure 1x2 trees have room above them (y-1) without bounds issues at 0
        for y in 2..MAP_H - 1 {
            for x in 1..MAP_W - 1 {
                if matches!(tiles[y][x], TileType::Grass) {
                    let r = hash_f(x as u32, y as u32, seed + 100);
                    if r < 0.08 {
                        objects.push(WorldObject { kind: ObjectKind::Tree, tile_x: x, tile_y: y, health: 3, alive: true });
                    } else if r < 0.10 {
                        objects.push(WorldObject { kind: ObjectKind::Rock, tile_x: x, tile_y: y, health: 5, alive: true });
                    }
                }
            }
        }

        // clear spawn area
        let (scx, scy) = (MAP_W / 2, MAP_H / 2);
        objects.retain(|o| {
            (o.tile_x as i32 - scx as i32).abs() > 3
                || (o.tile_y as i32 - scy as i32).abs() > 3
        });
        for dy in scy.saturating_sub(2)..=(scy + 2).min(MAP_H - 1) {
            for dx in scx.saturating_sub(2)..=(scx + 2).min(MAP_W - 1) {
                tiles[dy][dx] = TileType::Grass;
            }
        }

        // sort objects by y for depth rendering
        objects.sort_by_key(|o| o.tile_y);

        WorldMap { tiles, objects }
    }

    pub fn generate_tutorial() -> Self {
        // A simple tiny isolated island for tutorial
        let mut tiles = vec![vec![TileType::Water; MAP_W]; MAP_H];
        
        // Make a 20x20 grass island at (5, 5) -> (25, 25)
        for y in 5..=25 {
            for x in 5..=25 {
                tiles[y][x] = TileType::Grass;
            }
        }

        // Place exactly ONE tree for them to chop at (15, 10)
        let mut objects = Vec::new();
        objects.push(WorldObject { kind: ObjectKind::Tree, tile_x: 15, tile_y: 10, health: 3, alive: true });

        WorldMap { tiles, objects }
    }

    pub fn tile_at(&self, x: usize, y: usize) -> TileType {
        if x < MAP_W && y < MAP_H { self.tiles[y][x] } else { TileType::Water }
    }

    pub fn is_walkable(&self, x: usize, y: usize) -> bool {
        if x >= MAP_W || y >= MAP_H { return false; }
        if !self.tiles[y][x].is_walkable() { return false; }
        // dead objects don't block
        !self.objects.iter().any(|o| {
            if !o.alive { return false; }
            if o.kind == ObjectKind::Tree {
                // Tree is 1x2: it blocks its base (tile_y) and the tile above it (tile_y - 1)
                o.tile_x == x && (o.tile_y == y || (o.tile_y > 0 && o.tile_y - 1 == y))
            } else {
                o.tile_x == x && o.tile_y == y
            }
        })
    }

    pub fn object_index_at(&self, x: usize, y: usize) -> Option<usize> {
        self.objects.iter().position(|o| {
            if !o.alive { return false; }
            if o.kind == ObjectKind::Tree {
                o.tile_x == x && (o.tile_y == y || (o.tile_y > 0 && o.tile_y - 1 == y))
            } else {
                o.tile_x == x && o.tile_y == y
            }
        })
    }

    /// find the best adjacent walkable tile to stand on when interacting with (tx, ty)
    pub fn adjacent_walkable_tile(&self, tx: usize, ty: usize, hero_x: usize, hero_y: usize) -> Option<(usize, usize)> {
        let dirs: [(i32, i32); 4] = [(0, 1), (0, -1), (1, 0), (-1, 0)];
        let mut best: Option<(usize, usize)> = None;
        let mut best_dist = u32::MAX;
        for &(dx, dy) in &dirs {
            let nx = tx as i32 + dx;
            let ny = ty as i32 + dy;
            if nx < 0 || ny < 0 || nx >= MAP_W as i32 || ny >= MAP_H as i32 {
                continue;
            }
            let (ux, uy) = (nx as usize, ny as usize);
            if self.is_walkable(ux, uy) {
                let d = (hero_x as i32 - nx).unsigned_abs() + (hero_y as i32 - ny).unsigned_abs();
                if d < best_dist {
                    best_dist = d;
                    best = Some((ux, uy));
                }
            }
        }
        best
    }
}

pub struct Hero {
    pub world_x: f64,
    pub world_y: f64,
    pub path: Vec<(usize, usize)>,
    pub speed: f64,
    pub anim_timer: f64,
    pub anim_frame: u32,
    pub facing: u32,
    pub health: u32,
    pub max_health: u32,
    pub mana: u32,
    pub max_mana: u32,
}

impl Hero {
    pub fn new(tile_x: usize, tile_y: usize) -> Self {
        Hero {
            world_x: tile_x as f64 * TILE_SIZE as f64,
            world_y: tile_y as f64 * TILE_SIZE as f64,
            path: Vec::new(),
            speed: 0.08,
            anim_timer: 0.0,
            anim_frame: 0,
            facing: 0,
            health: 100,
            max_health: 100,
            mana: 50,
            max_mana: 50,
        }
    }

    pub fn tile_x(&self) -> usize { (self.world_x / TILE_SIZE as f64) as usize }
    pub fn tile_y(&self) -> usize { (self.world_y / TILE_SIZE as f64) as usize }

    pub fn update(&mut self, dt: f64) {
        if self.path.is_empty() {
            self.anim_frame = 0;
            return;
        }
        let (tx, ty) = self.path[0];
        let target_x = tx as f64 * TILE_SIZE as f64;
        let target_y = ty as f64 * TILE_SIZE as f64;
        let dx = target_x - self.world_x;
        let dy = target_y - self.world_y;
        let dist = (dx * dx + dy * dy).sqrt();
        let step = self.speed * dt;

        if dist <= step {
            self.world_x = target_x;
            self.world_y = target_y;
            self.path.remove(0);
        } else {
            self.world_x += dx / dist * step;
            self.world_y += dy / dist * step;
        }

        if dx.abs() > dy.abs() {
            self.facing = if dx > 0.0 { 3 } else { 2 };
        } else {
            self.facing = if dy > 0.0 { 0 } else { 1 };
        }

        self.anim_timer += dt;
        if self.anim_timer > 200.0 {
            self.anim_timer = 0.0;
            self.anim_frame = (self.anim_frame + 1) % 2;
        }
    }
}

pub struct Camera {
    pub x: f64,
    pub y: f64,
}

impl Camera {
    pub fn new() -> Self { Camera { x: 0.0, y: 0.0 } }

    pub fn follow(&mut self, hero: &Hero, vw: f64, vh: f64) {
        let tx = hero.world_x + TILE_SIZE as f64 * 0.5 - vw * 0.5;
        let ty = hero.world_y + TILE_SIZE as f64 * 0.5 - vh * 0.5;
        self.x += (tx - self.x) * 0.1;
        self.y += (ty - self.y) * 0.1;
        let mx = (MAP_W * TILE_SIZE) as f64 - vw;
        let my = (MAP_H * TILE_SIZE) as f64 - vh;
        self.x = self.x.max(0.0).min(mx);
        self.y = self.y.max(0.0).min(my);
    }

    pub fn snap_to(&mut self, hero: &Hero, vw: f64, vh: f64) {
        self.x = hero.world_x + TILE_SIZE as f64 * 0.5 - vw * 0.5;
        self.y = hero.world_y + TILE_SIZE as f64 * 0.5 - vh * 0.5;
        let mx = (MAP_W * TILE_SIZE) as f64 - vw;
        let my = (MAP_H * TILE_SIZE) as f64 - vh;
        self.x = self.x.max(0.0).min(mx);
        self.y = self.y.max(0.0).min(my);
    }
}
