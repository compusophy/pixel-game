use std::collections::HashMap;
use game_core::world::{WorldMap, Hero};
use game_core::protocol::{ServerMsg, PlayerSnapshot, ObjectUpdate, ClientMsg};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TutorialState {
    None,
    Move,
    Pickup,
    OpenInventory,
    Equip,
    Chop,
    Done,
}

pub struct ServerPlayer {
    pub id: u32,
    pub name: String,
    pub hero: Hero,
    pub is_tutorial: bool,
    pub tutorial_state: TutorialState,
}

pub struct GameWorld {
    pub map: WorldMap,
    pub map_seed: u32,
    pub players: HashMap<u32, ServerPlayer>,
    pub tutorial_instances: HashMap<u32, WorldMap>,
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
            tutorial_instances: HashMap::new(),
            next_player_id: 1,
        }
    }

    pub fn add_player(&mut self, name: String, is_tutorial: bool) -> u32 {
        let id = self.next_player_id;
        self.next_player_id += 1;
        
        let (spawn_x, spawn_y) = if is_tutorial {
            self.tutorial_instances.insert(id, WorldMap::generate_tutorial());
            (10, 10) // Spawn in the small tutorial box
        } else {
            (game_core::world::MAP_W / 2, game_core::world::MAP_H / 2)
        };

        let hero = Hero::new(spawn_x, spawn_y);
        let tutorial_state = if is_tutorial { TutorialState::Move } else { TutorialState::None };
        
        self.players.insert(id, ServerPlayer { 
            id, 
            name, 
            hero, 
            is_tutorial,
            tutorial_state,
        });
        id
    }

    pub fn remove_player(&mut self, id: u32) {
        self.players.remove(&id);
        self.tutorial_instances.remove(&id);
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
                    
                    let map = if p.is_tutorial {
                        self.tutorial_instances.get(&player_id).unwrap_or(&self.map)
                    } else {
                        &self.map
                    };

                    if tx < game_core::world::MAP_W && ty < game_core::world::MAP_H {
                        // find path
                        let start_tx = p.hero.tile_x();
                        let start_ty = p.hero.tile_y();
                        if tx != start_tx || ty != start_ty {
                            // if clicking an object, pathfind to adjacent tile
                            if let Some(obj_idx) = map.object_index_at(tx, ty) {
                                if let Some((adj_x, adj_y)) = map.adjacent_walkable_tile(tx, ty, start_tx, start_ty) {
                                    let path = game_core::pathfind::astar(map, (start_tx, start_ty), (adj_x, adj_y));
                                    if !path.is_empty() {
                                        p.hero.path = path;
                                        // TODO: handle chopping intent
                                        if p.is_tutorial && p.tutorial_state == TutorialState::Chop {
                                            if map.objects[obj_idx].kind == game_core::world::ObjectKind::Tree {
                                                p.tutorial_state = TutorialState::Done;
                                            }
                                        }
                                    }
                                }
                            } else if map.is_walkable(tx, ty) {
                                let path = game_core::pathfind::astar(map, (start_tx, start_ty), (tx, ty));
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

    pub fn tick(&mut self, dt: f64) -> (ServerMsg, Vec<(u32, ServerMsg)>) {
        let mut individual_messages = Vec::new();

        // update all players
        for p in self.players.values_mut() {
            p.hero.update(dt);

            // TUTORIAL PROGRESSION LOGIC
            if p.is_tutorial {
                match p.tutorial_state {
                    TutorialState::Move => {
                        let dist_from_spawn = (p.hero.tile_x() as i32 - 10).abs() + (p.hero.tile_y() as i32 - 10).abs();
                        if dist_from_spawn > 3 {
                            p.tutorial_state = TutorialState::Pickup;
                        } else {
                            individual_messages.push((p.id, ServerMsg::FloatingText {
                                x: p.hero.world_x,
                                y: p.hero.world_y - 20.0,
                                text: "Click to move!".to_string(),
                                color: (255, 255, 0),
                            }));
                        }
                    }
                    TutorialState::Pickup => {
                        // For now we skip inventory systems and just let them walk to the tree
                        let dist_to_tree = (p.hero.tile_x() as i32 - 15).abs() + (p.hero.tile_y() as i32 - 10).abs();
                        if dist_to_tree <= 2 {
                            p.tutorial_state = TutorialState::Chop; // Skipping Equip stage since there's no item system on server yet
                        } else {
                            individual_messages.push((p.id, ServerMsg::FloatingText {
                                x: p.hero.world_x,
                                y: p.hero.world_y - 20.0,
                                text: "Walk to the Tree (right)".to_string(),
                                color: (255, 255, 0),
                            }));
                        }
                    }
                    TutorialState::Chop => {
                        // Check if they clicked the tree
                        // We can't perfectly check clicks here, but we can instruct them
                        individual_messages.push((p.id, ServerMsg::FloatingText {
                            x: p.hero.world_x,
                            y: p.hero.world_y - 20.0,
                            text: "Click the Tree to Chop!".to_string(),
                            color: (255, 200, 50),
                        }));
                    }
                    TutorialState::Done => {
                        individual_messages.push((p.id, ServerMsg::FloatingText {
                            x: p.hero.world_x,
                            y: p.hero.world_y - 20.0,
                            text: "Tutorial Complete! Refresh page to join multiplayer.".to_string(),
                            color: (0, 255, 0),
                        }));
                    }
                    _ => {}
                }
            }
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

        (ServerMsg::Tick {
            players: snapshots,
            objects,
        }, individual_messages)
    }
}
