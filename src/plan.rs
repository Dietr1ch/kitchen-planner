use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Plan {
    pub start_time: String,
    pub tasks: Vec<Task>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Task {
    pub description: String,
    pub start_offset_minutes: u32,
    pub duration_minutes: u32,
    #[serde(default)]
    pub resource_id: Option<String>,
    #[serde(default)]
    pub cook: Option<String>,
}
