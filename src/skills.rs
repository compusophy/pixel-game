pub struct WoodcuttingSkill {
    pub xp: u32,
    pub level: u32,
}

impl WoodcuttingSkill {
    pub fn new() -> Self {
        WoodcuttingSkill { xp: 0, level: 1 }
    }

    pub fn add_xp(&mut self, amount: u32) {
        self.xp += amount;
        // recalculate level
        while self.xp >= self.xp_for_next_level() {
            self.level += 1;
        }
    }

    /// xp threshold to reach a given level
    pub fn xp_for_level(level: u32) -> u32 {
        if level <= 1 {
            return 0;
        }
        // simple curve: level 2 = 100, level 3 = 250, level 4 = 475, etc.
        // formula: sum of (75 * (l-1) + 25) for l in 2..=level
        let mut total = 0u32;
        for l in 2..=level {
            total += 75 * (l - 1) + 25;
        }
        total
    }

    pub fn xp_for_next_level(&self) -> u32 {
        Self::xp_for_level(self.level + 1)
    }

    pub fn xp_in_current_level(&self) -> u32 {
        self.xp - Self::xp_for_level(self.level)
    }

    pub fn xp_needed_for_next(&self) -> u32 {
        self.xp_for_next_level() - Self::xp_for_level(self.level)
    }
}
