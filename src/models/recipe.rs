use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Recipe {
	pub name: String,
	pub ingredients: Vec<Ingredient>,
	pub steps: Vec<Step>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Ingredient {
	pub name: String,
	pub quantity: f64,
	pub unit: String,
	#[serde(default)]
	pub optional: bool,
	#[serde(default)]
	pub alternatives: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Step {
	pub id: String,
	pub description: String,
	pub duration_minutes: u32,
	#[serde(default)]
	pub resource_kind: Option<String>,
	#[serde(default)]
	pub dependencies: Vec<String>,
	#[serde(default)]
	pub optional: bool,
	#[serde(default = "default_true")]
	pub needs_cook: bool,
}

fn default_true() -> bool {
	true
}
