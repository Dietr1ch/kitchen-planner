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

	/// How long the task takes
	/// Skill-dependent tasks override this with duration_by_skill
	// TODO: Use an enum for TaskDuration to make this alternative properly typed.
	pub duration_minutes: u32,

	#[serde(default)]
	pub resource_kinds: Vec<String>,
	#[serde(default)]
	pub dependencies: Vec<String>,
	#[serde(default)]
	pub optional: bool,

	/// Whether a cook needs to perform this step
	///
	/// This is useful to let some steps like pre-heating an oven or cooling down
	/// progress on their own without active supervision
	#[serde(default = "default_true")]
	pub needs_cook: bool,

	/// Whether this steps needs a particular skill
	#[serde(default)]
	pub skill: Option<String>,
	/// Whether requires a certain proficiency in said skill
	#[serde(default)]
	pub min_skill_level: Option<SkillLevel>,
	/// How long the task takes based on the cook's skill level
	#[serde(default)]
	pub duration_by_skill: Option<HashMap<SkillLevel, u32>>,

	/// Temperature at which the step should take place.
	/// Useful for repeatable baking, grilling or flash freezing.
	#[serde(default)]
	pub temperature_celsius: Option<u16>,
}

fn default_true() -> bool {
	true
}
