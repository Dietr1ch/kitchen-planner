use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process;

use clap::Parser;
use kitchen_planner::models::plan::Plan;
use kitchen_planner::render::{Renderer, SortOrder, TextRenderer};

#[derive(Parser)]
#[command(name = "gantt")]
struct Cli {
	path: PathBuf,
	#[arg(short, long, value_enum, default_value_t = SortOrder::Start)]
	sort_by: SortOrder,
}

fn main() {
	let cli = Cli::parse();

	let resolved = if cli.path == Path::new("-") {
		Path::new("/dev/stdin")
	} else {
		&cli.path
	};

	let content = fs::read_to_string(resolved).unwrap_or_else(|e| {
		eprintln!("Error reading {}: {}", cli.path.display(), e);
		process::exit(1);
	});

	let plan: Plan = serde_json::from_str(&content).unwrap_or_else(|e| {
		eprintln!("Invalid plan: {}", e);
		process::exit(1);
	});

	let renderer = TextRenderer::new(cli.sort_by);
	let output = renderer.render(&plan);
	let stdout = io::stdout();
	let mut handle = stdout.lock();
	let _ = write!(handle, "{}", output);
	let _ = handle.flush();
}
