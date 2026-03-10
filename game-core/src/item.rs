use std::collections::HashMap;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
#[repr(u32)]
pub enum ItemId {
    Logs = 1,
    Stone = 2,
}

impl ItemId {
    pub fn name(self) -> &'static str {
        match self {
            ItemId::Logs => "logs",
            ItemId::Stone => "stone",
        }
    }

    pub fn from_u32(v: u32) -> Option<Self> {
        match v {
            1 => Some(ItemId::Logs),
            2 => Some(ItemId::Stone),
            _ => None,
        }
    }
}

pub struct Inventory {
    items: HashMap<ItemId, u32>,
}

impl Inventory {
    pub fn new() -> Self {
        Inventory {
            items: HashMap::new(),
        }
    }

    pub fn add(&mut self, id: ItemId, qty: u32) -> u32 {
        let entry = self.items.entry(id).or_insert(0);
        *entry += qty;
        *entry
    }

    pub fn remove(&mut self, id: ItemId, qty: u32) -> Option<u32> {
        let entry = self.items.get_mut(&id)?;
        if *entry < qty {
            return None;
        }
        *entry -= qty;
        if *entry == 0 {
            self.items.remove(&id);
            Some(0)
        } else {
            Some(*entry)
        }
    }

    pub fn count(&self, id: ItemId) -> u32 {
        self.items.get(&id).copied().unwrap_or(0)
    }

    /// returns flat pairs: [item_id_u32, qty, item_id_u32, qty, ...]
    pub fn to_flat_vec(&self) -> Vec<u32> {
        let mut out = Vec::with_capacity(self.items.len() * 2);
        // sort by item id for stable ordering
        let mut entries: Vec<_> = self.items.iter().collect();
        entries.sort_by_key(|(id, _)| **id as u32);
        for (id, qty) in entries {
            out.push(*id as u32);
            out.push(*qty);
        }
        out
    }
}
