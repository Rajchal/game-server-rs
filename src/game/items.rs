use serde::{Deserialize, Serialize};

/// The three holy items a player must collect to find and slay the dragon.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Item {
    HolySword,
    HolyArmor,
    DragonMap,
}

impl Item {
    /// Returns all three items.
    pub fn all() -> [Item; 3] {
        [Item::HolySword, Item::HolyArmor, Item::DragonMap]
    }
}

/// An item placed in the world at a specific tile coordinate.
#[derive(Debug, Clone)]
pub struct ItemSpawn {
    pub item: Item,
    pub x: i32,
    pub y: i32,
}
