use std::fs;
use std::path::{Path, PathBuf};
use std::process;

use clap::Parser;

use kitchen_planner::cook::Cook;
use kitchen_planner::kitchen::Kitchen;
use kitchen_planner::plan::Plan;
use kitchen_planner::recipe::Recipe;

#[derive(Parser)]
#[command(name = "kitchen-planner")]
enum Cli {
    /// Validate and display a kitchen schema
    Kitchen { path: PathBuf },
    /// Validate and display a recipe schema
    Recipe { path: PathBuf },
    /// Validate and display a cook schema
    Cook { path: PathBuf },
    /// Validate and display a plan schema
    Plan { path: PathBuf },
    /// Generate a meal plan from kitchen, cooks directory, and recipes
    Schedule {
        kitchen: PathBuf,
        cooks_dir: PathBuf,
        recipes: Vec<PathBuf>,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli {
        Cli::Kitchen { path } => process_file::<Kitchen>("kitchen", &path),
        Cli::Recipe { path } => process_file::<Recipe>("recipe", &path),
        Cli::Cook { path } => process_file::<Cook>("cook", &path),
        Cli::Plan { path } => process_file::<Plan>("plan", &path),
        Cli::Schedule {
            kitchen,
            cooks_dir,
            recipes,
        } => schedule(kitchen, cooks_dir, recipes),
    }
}

fn resolve_path(path: &Path) -> &Path {
    if path == Path::new("-") {
        Path::new("/dev/stdin")
    } else {
        path
    }
}

fn process_file<T: serde::Serialize + serde::de::DeserializeOwned>(
    schema: &str,
    path: &Path,
) {
    let content = match fs::read_to_string(resolve_path(path)) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error reading {}: {}", path.display(), e);
            process::exit(1);
        }
    };

    let value: T = serde_json::from_str(&content).unwrap_or_else(|e| {
        eprintln!("Invalid {} schema: {}", schema, e);
        process::exit(1);
    });

    println!("{}", serde_json::to_string_pretty(&value).unwrap());
}

fn schedule(kitchen_path: PathBuf, cooks_dir: PathBuf, recipe_paths: Vec<PathBuf>) {
    let kitchen: Kitchen = read_file("kitchen", &kitchen_path);

    let mut cook_files: Vec<PathBuf> = std::fs::read_dir(&cooks_dir)
        .unwrap_or_else(|e| {
            eprintln!("Error reading cooks directory {}: {}", cooks_dir.display(), e);
            process::exit(1);
        })
        .filter_map(|entry| {
            let path = entry.ok()?.path();
            (path.extension() == Some("json".as_ref())).then_some(path)
        })
        .collect();
    cook_files.sort();

    let cooks: Vec<Cook> = cook_files.iter().map(|p| read_file("cook", p)).collect();

    let recipes: Vec<Recipe> = recipe_paths
        .iter()
        .map(|p| read_file("recipe", p))
        .collect();

    let plan = kitchen_planner::schedule::schedule(&kitchen, &cooks, &recipes);
    println!("{}", serde_json::to_string_pretty(&plan).unwrap());
}

fn read_file<T: serde::de::DeserializeOwned>(schema: &str, path: &Path) -> T {
    let content = fs::read_to_string(resolve_path(path)).unwrap_or_else(|e| {
        eprintln!("Error reading {}: {}", path.display(), e);
        process::exit(1);
    });
    serde_json::from_str(&content).unwrap_or_else(|e| {
        eprintln!("Invalid {} schema in {}: {}", schema, path.display(), e);
        process::exit(1);
    })
}

