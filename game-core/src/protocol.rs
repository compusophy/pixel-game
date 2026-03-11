use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ClientMsg {
    Join { name: String, is_tutorial: bool },
    Click { world_x: f64, world_y: f64 },
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PlayerSnapshot {
    pub id: u32,
    pub name: String,
    pub world_x: f64,
    pub world_y: f64,
    pub facing: u32,
    pub anim_frame: u32,
    pub is_moving: bool,
    pub health: i32,
    pub max_health: i32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ObjectUpdate {
    pub map_index: usize,
    pub alive: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ServerMsg {
    Welcome {
        player_id: u32,
        map_seed: u32,
        is_tutorial: bool,
    },
    Tick {
        players: Vec<PlayerSnapshot>,
        objects: Vec<ObjectUpdate>,
    },
    FloatingText {
        x: f64,
        y: f64,
        text: String,
        color: (u8, u8, u8),
    },
}
