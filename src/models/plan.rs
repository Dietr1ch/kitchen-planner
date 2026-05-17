use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, ts_rs::TS)]
#[ts(export)]
pub struct Plan {
	pub tasks: Vec<Task>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ts_rs::TS)]
#[ts(export)]
pub struct Task {
	#[serde(default)]
	pub id: String,
	#[serde(default)]
	pub dish: String,
	pub description: String,
	pub start_offset_minutes: u32,
	pub duration_minutes: u32,
	#[serde(default)]
	pub resource_ids: Vec<Option<String>>,
	#[serde(default)]
	pub resource_kinds: Vec<String>,
	#[serde(default)]
	pub cook: Option<String>,
	#[serde(default)]
	pub dependencies: Vec<String>,
}
