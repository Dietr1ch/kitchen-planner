use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Cook {
    pub name: String,
    pub knife_skill: KnifeSkill,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum KnifeSkill {
    Novice,
    Intermediate,
    Advanced,
    Expert,
}
