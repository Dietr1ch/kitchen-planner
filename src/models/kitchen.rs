use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Kitchen {
	pub equipment: Vec<Equipment>,
	#[serde(default = "default_ambient")]
	pub ambient_temperature_celsius: f64,
	pub food: Vec<Food>,
	pub materials: Vec<Material>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Equipment {
	pub id: String,
	pub name: String,
	pub kind: String,
	#[serde(default)]
	pub preheat_rate_minutes_per_celsius: f64,
}

fn default_ambient() -> f64 {
	20.0
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
