use std::path::{Path, PathBuf};

use axum::{
	Router,
	extract::State,
	http::StatusCode,
	response::IntoResponse,
	routing::{get, post},
};
use tower_http::services::ServeDir;

use crate::io;
use crate::models::cook::Cook;
use crate::models::kitchen::Kitchen;
use crate::models::recipe::Recipe;

#[derive(Clone)]
pub struct AppState {
	pub default_kitchen: PathBuf,
	pub default_cooks_dir: PathBuf,
	pub default_recipes_dir: PathBuf,
}

#[derive(serde::Serialize)]
pub struct DefaultsResponse {
	pub kitchen: Kitchen,
	pub cooks: Vec<Cook>,
	pub recipes: Vec<Recipe>,
}

#[derive(serde::Deserialize)]
pub struct PlanRequest {
	pub kitchen: Kitchen,
	pub cooks: Vec<Cook>,
	pub recipes: Vec<Recipe>,
}

pub fn create_router(state: AppState, assets_path: impl AsRef<Path>) -> Router {
	let assets = assets_path.as_ref().to_path_buf();
	Router::new()
		.route("/api/defaults", get(defaults_handler))
		.route("/api/plan", post(plan_handler))
		.fallback_service(ServeDir::new(assets).append_index_html_on_directories(true))
		.with_state(state)
}

async fn defaults_handler(State(state): State<AppState>) -> impl IntoResponse {
	let kitchen: Kitchen = match io::read_ron_file(&state.default_kitchen) {
		Ok(k) => k,
		Err(e) => {
			return (
				StatusCode::INTERNAL_SERVER_ERROR,
				serde_json::json!({"error": e.to_string()}).to_string(),
			);
		}
	};

	let cooks_dir = std::fs::read_dir(&state.default_cooks_dir).map(|entries| {
		entries
			.filter_map(|e| e.ok())
			.map(|e| e.path())
			.filter(|p| p.extension().is_some_and(|ext| ext == "ron"))
			.collect::<Vec<_>>()
	});

	let cooks: Vec<Cook> = match cooks_dir {
		Ok(paths) => match io::read_ron_files(&paths) {
			Ok(c) => c,
			Err(e) => {
				return (
					StatusCode::INTERNAL_SERVER_ERROR,
					serde_json::json!({"error": e.to_string()}).to_string(),
				);
			}
		},
		Err(e) => {
			return (
				StatusCode::INTERNAL_SERVER_ERROR,
				serde_json::json!({"error": e.to_string()}).to_string(),
			);
		}
	};

	let recipes_dir = std::fs::read_dir(&state.default_recipes_dir).map(|entries| {
		entries
			.filter_map(|e| e.ok())
			.map(|e| e.path())
			.filter(|p| p.extension().is_some_and(|ext| ext == "ron"))
			.collect::<Vec<_>>()
	});

	let recipes: Vec<Recipe> = match recipes_dir {
		Ok(paths) => match io::read_ron_files(&paths) {
			Ok(r) => r,
			Err(e) => {
				return (
					StatusCode::INTERNAL_SERVER_ERROR,
					serde_json::json!({"error": e.to_string()}).to_string(),
				);
			}
		},
		Err(e) => {
			return (
				StatusCode::INTERNAL_SERVER_ERROR,
				serde_json::json!({"error": e.to_string()}).to_string(),
			);
		}
	};

	let response = DefaultsResponse {
		kitchen,
		cooks,
		recipes,
	};
	match serde_json::to_string(&response) {
		Ok(json) => (StatusCode::OK, json),
		Err(e) => (
			StatusCode::INTERNAL_SERVER_ERROR,
			serde_json::json!({"error": e.to_string()}).to_string(),
		),
	}
}

async fn plan_handler(
	State(_state): State<AppState>,
	axum::Json(req): axum::Json<PlanRequest>,
) -> impl IntoResponse {
	let kitchen = req.kitchen;
	let cooks = req.cooks;
	let recipes = req.recipes;

	let result =
		tokio::task::spawn_blocking(move || crate::schedule::schedule(&kitchen, &cooks, &recipes))
			.await;

	match result {
		Ok(Ok(plan)) => match serde_json::to_string(&plan) {
			Ok(json) => (StatusCode::OK, json),
			Err(e) => (
				StatusCode::INTERNAL_SERVER_ERROR,
				serde_json::json!({"error": e.to_string()}).to_string(),
			),
		},
		Ok(Err(e)) => (
			StatusCode::INTERNAL_SERVER_ERROR,
			serde_json::json!({"error": e.to_string()}).to_string(),
		),
		Err(e) => (
			StatusCode::INTERNAL_SERVER_ERROR,
			serde_json::json!({"error": format!("solver task panicked: {}", e)}).to_string(),
		),
	}
}
