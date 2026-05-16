use std::path::PathBuf;

use clap::Parser;

use kitchen_planner::web::{AppState, create_router};

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

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
	color_eyre::install()?;
	let args = Cli::parse();

	let state = AppState {
		default_kitchen: args.default_kitchen,
		default_cooks_dir: args.default_cooks_dir,
		default_recipes_dir: args.default_recipes_dir,
	};

	let app = create_router(state, &args.assets);

	let addr = format!("{}:{}", args.host, args.port);
	println!("Listening on http://{}", addr);
	let listener = tokio::net::TcpListener::bind(&addr).await?;
	axum::serve(listener, app).await?;

	Ok(())
}
