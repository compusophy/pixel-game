use std::collections::HashMap;
use game_core::world::{WorldMap, Hero};
use game_core::protocol::{ServerMsg, PlayerSnapshot, ObjectUpdate, ClientMsg};

pub struct ServerPlayer {
    pub id: u32,
    pub name: String,
    pub hero: Hero,
}

pub struct GameWorld {
    pub map: WorldMap,
    pub map_seed: u32,
    pub players: HashMap<u32, ServerPlayer>,
    pub next_player_id: u32,
}

impl GameWorld {
    pub fn new() -> Self {
        let seed = 12345;
        let map = WorldMap::generate(seed);
        GameWorld {
            map,
            map_seed: seed,
            players: HashMap::new(),
            next_player_id: 1,
        }
    }

    pub fn add_player(&mut self, name: String) -> u32 {
        let id = self.next_player_id;
        self.next_player_id += 1;
        // spawn in the middle of the map
        let hero = Hero::new(game_core::world::MAP_W / 2, game_core::world::MAP_H / 2);
        self.players.insert(id, ServerPlayer { id, name, hero });
        id
    }

    pub fn remove_player(&mut self, id: u32) {
        self.players.remove(&id);
    }

    pub fn handle_client_msg(&mut self, player_id: u32, msg: ClientMsg) {
        match msg {
            ClientMsg::Join { .. } => {
                // handled in connection setup
            }
            ClientMsg::Click { world_x, world_y } => {
                if let Some(p) = self.players.get_mut(&player_id) {
                    let tx = (world_x / game_core::world::TILE_SIZE as f64).floor() as usize;
                    let ty = (world_y / game_core::world::TILE_SIZE as f64).floor() as usize;
                    if tx < game_core::world::MAP_W && ty < game_core::world::MAP_H {
                        // find path
                        let start_tx = p.hero.tile_x();
                        let start_ty = p.hero.tile_y();
                        if tx != start_tx || ty != start_ty {
                            // if clicking an object, pathfind to adjacent tile
                            if let Some(obj_idx) = self.map.object_index_at(tx, ty) {
                                if let Some((adj_x, adj_y)) = self.map.adjacent_walkable_tile(tx, ty, start_tx, start_ty) {
                                    let path = game_core::pathfind::astar(&self.map, (start_tx, start_ty), (adj_x, adj_y));
                                    if !path.is_empty() {
                                        p.hero.path = path;
                                        // TODO: handle chopping intent
                                    }
                                }
                            } else if self.map.is_walkable(tx, ty) {
                                let path = game_core::pathfind::astar(&self.map, (start_tx, start_ty), (tx, ty));
                                if !path.is_empty() {
                                    p.hero.path = path;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    pub fn tick(&mut self, dt: f64) -> ServerMsg {
        // update all players
        for p in self.players.values_mut() {
            p.hero.update(dt);
        }

        // build snapshot
        let mut snapshots = Vec::with_capacity(self.players.len());
        for p in self.players.values() {
            snapshots.push(PlayerSnapshot {
                id: p.id,
                name: p.name.clone(),
                world_x: p.hero.world_x,
                world_y: p.hero.world_y,
                facing: p.hero.facing,
                anim_frame: p.hero.anim_frame,
                is_moving: !p.hero.path.is_empty(),
                health: p.hero.health as i32,
                max_health: p.hero.max_health as i32,
            });
        }

        // TODO: track dirty objects to only send updates, for now send none or all
        let objects = Vec::new();

        ServerMsg::Tick {
            players: snapshots,
            objects,
        }
    }
}
