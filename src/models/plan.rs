use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Plan {
    pub start_time: String,
    pub tasks: Vec<Task>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Task {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub dish: String,
    pub description: String,
    pub start_offset_minutes: u32,
    pub duration_minutes: u32,
    #[serde(default)]
    pub resource_id: Option<String>,
    #[serde(default)]
    pub resource_kind: Option<String>,
    #[serde(default)]
    pub cook: Option<String>,
    #[serde(default)]
    pub dependencies: Vec<String>,
}
