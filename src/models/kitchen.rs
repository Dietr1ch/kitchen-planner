use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Kitchen {
    pub equipment: Vec<Equipment>,
    pub food: Vec<Food>,
    pub materials: Vec<Material>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Equipment {
    pub id: String,
    pub name: String,
    pub kind: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Food {
    pub id: String,
    pub name: String,
    pub quantity: f64,
    pub unit: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Material {
    pub id: String,
    pub name: String,
    pub quantity: f64,
    pub unit: String,
}
