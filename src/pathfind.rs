use std::collections::BinaryHeap;
use std::collections::HashMap;
use std::cmp::Ordering;

use crate::world::{WorldMap, MAP_W, MAP_H};

#[derive(Clone, Eq, PartialEq)]
struct Node {
    tile: (usize, usize),
    cost: u32,
    heuristic: u32,
}

impl Ord for Node {
    fn cmp(&self, other: &Self) -> Ordering {
        (other.cost + other.heuristic).cmp(&(self.cost + self.heuristic))
    }
}

impl PartialOrd for Node {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

pub fn astar(
    map: &WorldMap,
    start: (usize, usize),
    goal: (usize, usize),
) -> Vec<(usize, usize)> {
    if start == goal || !map.is_walkable(goal.0, goal.1) {
        return Vec::new();
    }

    let mut open = BinaryHeap::new();
    let mut came_from: HashMap<(usize, usize), (usize, usize)> = HashMap::new();
    let mut g_score: HashMap<(usize, usize), u32> = HashMap::new();

    g_score.insert(start, 0);
    open.push(Node {
        tile: start,
        cost: 0,
        heuristic: heuristic(start, goal),
    });

    let dirs: [(i32, i32); 4] = [(0, -1), (0, 1), (-1, 0), (1, 0)];

    while let Some(current) = open.pop() {
        if current.tile == goal {
            let mut path = Vec::new();
            let mut c = goal;
            while c != start {
                path.push(c);
                c = came_from[&c];
            }
            path.reverse();
            return path;
        }

        let cg = g_score[&current.tile];

        for &(dx, dy) in &dirs {
            let nx = current.tile.0 as i32 + dx;
            let ny = current.tile.1 as i32 + dy;
            if nx < 0 || ny < 0 || nx >= MAP_W as i32 || ny >= MAP_H as i32 {
                continue;
            }
            let next = (nx as usize, ny as usize);
            if !map.is_walkable(next.0, next.1) {
                continue;
            }
            let new_g = cg + 10;
            if new_g < g_score.get(&next).copied().unwrap_or(u32::MAX) {
                g_score.insert(next, new_g);
                came_from.insert(next, current.tile);
                open.push(Node {
                    tile: next,
                    cost: new_g,
                    heuristic: heuristic(next, goal),
                });
            }
        }
    }

    Vec::new()
}

fn heuristic(a: (usize, usize), b: (usize, usize)) -> u32 {
    let dx = (a.0 as i32 - b.0 as i32).unsigned_abs();
    let dy = (a.1 as i32 - b.1 as i32).unsigned_abs();
    (dx + dy) * 10
}
