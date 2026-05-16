use std::io::Write;
use std::path::PathBuf;

use clap::Parser;

use kitchen_planner::io;
use kitchen_planner::models::plan::Plan;
use kitchen_planner::render::{Renderer, SortOrder, TextRenderer};

#[derive(Parser)]
#[command(name = "gantt")]
struct Cli {
	#[arg(short, long, value_enum, default_value_t = SortOrder::Start)]
	sort_by: SortOrder,

	plan_path: PathBuf,
}

fn main() -> color_eyre::Result<()> {
	use color_eyre::eyre::WrapErr;
	let args = Cli::parse();

	let plan: Plan = io::read_json_file(&args.plan_path).wrap_err("Failed to read Plan file")?;

	let renderer = TextRenderer::new(args.sort_by);
	let output = renderer.render(&plan);
	let stdout = std::io::stdout();
	let mut handle = stdout.lock();
	let _ = write!(handle, "{}", output);
	let _ = handle.flush();

	Ok(())
}
