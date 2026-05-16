use std::path::PathBuf;

use clap::Parser;

use kitchen_planner::io;
use kitchen_planner::models::cook::Cook;
use kitchen_planner::models::kitchen::Kitchen;
use kitchen_planner::models::recipe::Recipe;

#[derive(Parser)]
#[command(name = "schedule")]
struct Cli {
	#[arg(long)]
	kitchen: PathBuf,
	#[arg(long="cook")]
	cooks: Vec<PathBuf>,
	recipes: Vec<PathBuf>,
}

fn main() -> color_eyre::Result<()> {
	use color_eyre::eyre::WrapErr;
	let args = Cli::parse();

	let kitchen: Kitchen =
		io::read_ron_file(&args.kitchen).wrap_err("Failed to read Kitchen RON file")?;
	let cooks: Vec<Cook> =
		io::read_ron_files(&args.cooks).wrap_err("Failed to read Cook RON files")?;
	let recipes: Vec<Recipe> =
		io::read_ron_files(&args.recipes).wrap_err("Failed to read Recipe RON files")?;

	let plan = kitchen_planner::schedule::schedule(&kitchen, &cooks, &recipes);
	println!("{}", serde_json::to_string_pretty(&plan).unwrap());

	Ok(())
}
