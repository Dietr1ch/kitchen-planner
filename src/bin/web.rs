use std::path::PathBuf;

use axum::{
	Router,
	extract::State,
	http::StatusCode,
	response::IntoResponse,
	routing::{get, post},
};
use clap::Parser;
use tower_http::services::ServeDir;

use kitchen_planner::io;
use kitchen_planner::models::cook::Cook;
use kitchen_planner::models::kitchen::Kitchen;
use kitchen_planner::models::recipe::Recipe;

#[derive(Parser)]
#[command(name = "web")]
struct Cli {
	#[arg(long, default_value = "0.0.0.0")]
	host: String,

	#[arg(long, default_value = "3004")]
	port: u16,

	#[arg(long, default_value = "assets")]
	assets: PathBuf,

	#[arg(long = "kitchen", default_value = "data/kitchens/simple.ron")]
	default_kitchen: PathBuf,

	#[arg(long = "cook", default_value = "data/cooks")]
	default_cooks_dir: PathBuf,

	#[arg(long = "recipe", default_value = "data/recipes")]
	default_recipes_dir: PathBuf,
}

#[derive(Clone)]
struct AppState {
	default_kitchen: PathBuf,
	default_cooks_dir: PathBuf,
	default_recipes_dir: PathBuf,
}

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
	color_eyre::install()?;
	let args = Cli::parse();

	let state = AppState {
		default_kitchen: args.default_kitchen,
		default_cooks_dir: args.default_cooks_dir,
		default_recipes_dir: args.default_recipes_dir,
	};

	let app = Router::new()
		.route("/api/defaults", get(defaults_handler))
		.route("/api/plan", post(plan_handler))
		.fallback_service(ServeDir::new(&args.assets).append_index_html_on_directories(true))
		.with_state(state);

	let addr = format!("{}:{}", args.host, args.port);
	println!("Listening on http://{}", addr);
	let listener = tokio::net::TcpListener::bind(&addr).await?;
	axum::serve(listener, app).await?;

	Ok(())
}

#[derive(serde::Serialize)]
struct DefaultsResponse {
	kitchen: Kitchen,
	cooks: Vec<Cook>,
	recipes: Vec<Recipe>,
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

#[derive(serde::Deserialize)]
struct PlanRequest {
	kitchen: Kitchen,
	cooks: Vec<Cook>,
	recipes: Vec<Recipe>,
}

async fn plan_handler(
	State(_state): State<AppState>,
	axum::Json(req): axum::Json<PlanRequest>,
) -> impl IntoResponse {
	let kitchen = req.kitchen;
	let cooks = req.cooks;
	let recipes = req.recipes;

	let result = tokio::task::spawn_blocking(move || {
		kitchen_planner::schedule::schedule(&kitchen, &cooks, &recipes)
	})
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
