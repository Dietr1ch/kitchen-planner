use std::fs;
use std::process;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 {
        eprintln!("Usage: kitchen-planner <schema> <file>");
        eprintln!("  schema: kitchen, recipe, cook, plan");
        process::exit(1);
    }

    let schema = &args[1];
    let path = &args[2];

    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error reading {}: {}", path, e);
            process::exit(1);
        }
    };

    match schema.as_str() {
        "kitchen" => {
            let kitchen: kitchen_planner::kitchen::Kitchen =
                serde_json::from_str(&content).unwrap_or_else(|e| {
                    eprintln!("Invalid kitchen schema: {}", e);
                    process::exit(1);
                });
            println!("{}", serde_json::to_string_pretty(&kitchen).unwrap());
        }
        "recipe" => {
            let recipe: kitchen_planner::recipe::Recipe =
                serde_json::from_str(&content).unwrap_or_else(|e| {
                    eprintln!("Invalid recipe schema: {}", e);
                    process::exit(1);
                });
            println!("{}", serde_json::to_string_pretty(&recipe).unwrap());
        }
        "cook" => {
            let cook: kitchen_planner::cook::Cook =
                serde_json::from_str(&content).unwrap_or_else(|e| {
                    eprintln!("Invalid cook schema: {}", e);
                    process::exit(1);
                });
            println!("{}", serde_json::to_string_pretty(&cook).unwrap());
        }
        "plan" => {
            let plan: kitchen_planner::plan::Plan =
                serde_json::from_str(&content).unwrap_or_else(|e| {
                    eprintln!("Invalid plan schema: {}", e);
                    process::exit(1);
                });
            println!("{}", serde_json::to_string_pretty(&plan).unwrap());
        }
        _ => {
            eprintln!(
                "Unknown schema: {}. Use: kitchen, recipe, cook, plan",
                schema
            );
            process::exit(1);
        }
    }
}
