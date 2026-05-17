use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::cook::SkillLevel;

#[derive(Debug, Serialize, Deserialize, ts_rs::TS)]
#[ts(export)]
pub struct Recipe {
	pub name: String,
	pub ingredients: Vec<Ingredient>,
	pub steps: Vec<Step>,
}

#[derive(Debug, Serialize, Deserialize, ts_rs::TS)]
#[ts(export)]
pub struct Ingredient {
	pub name: String,
	pub quantity: f64,
	pub unit: String,
	#[serde(default)]
	pub optional: bool,
	#[serde(default)]
	pub alternatives: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ts_rs::TS)]
#[ts(export)]
pub struct Step {
	pub id: String,
	pub description: String,
	pub duration_minutes: u32,
	#[serde(default)]
	pub resource_kinds: Vec<String>,
	#[serde(default)]
	pub dependencies: Vec<String>,
	#[serde(default)]
	pub optional: bool,
	#[serde(default = "default_true")]
	pub needs_cook: bool,
	#[serde(default)]
	pub skill: Option<String>,
	#[serde(default)]
	pub min_skill_level: Option<SkillLevel>,
	#[serde(default)]
	pub duration_by_skill: Option<HashMap<SkillLevel, u32>>,
	#[serde(default)]
	pub temperature_celsius: Option<u16>,
}

fn default_true() -> bool {
	true
}
