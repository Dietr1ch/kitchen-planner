use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process;

use clap::{Parser, ValueEnum};
use kitchen_planner::plan::Plan;
use kitchen_planner::render::{HtmlRenderer, Renderer, TextRenderer};

#[derive(Parser)]
#[command(name = "gantt")]
struct Cli {
	path: PathBuf,
	#[arg(short, long, value_enum, default_value_t = Format::Text)]
	format: Format,
}

#[derive(Copy, Clone, ValueEnum)]
enum Format {
	Text,
	Html,
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

	let renderer: Box<dyn Renderer> = match cli.format {
		Format::Text => Box::new(TextRenderer),
		Format::Html => Box::new(HtmlRenderer),
	};

	let output = renderer.render(&plan);
	let stdout = io::stdout();
	let mut handle = stdout.lock();
	let _ = write!(handle, "{}", output);
	let _ = handle.flush();
}
