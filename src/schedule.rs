use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::Write;
use std::process::{Command, Stdio};

use crate::cook::Cook;
use crate::kitchen::Kitchen;
use crate::plan::{Plan, Task};
use crate::recipe::Recipe;

fn needs_cook(resource_id: &Option<String>, kitchen: &Kitchen) -> bool {
    match resource_id {
        None => false,
        Some(rid) => kitchen
            .equipment
            .iter()
            .find(|e| e.id == *rid)
            .map(|e| e.kind != "oven")
            .unwrap_or(true),
    }
}

fn build_model(
    durations: &[u32],
    resources: &[usize],
    needs_cook_arr: &[bool],
    recipe_of: &[usize],
    deps_from: &[usize],
    deps_to: &[usize],
    num_cooks: usize,
    num_recipes: usize,
    horizon: u32,
) -> String {
    let num_tasks = durations.len();
    let num_deps = deps_from.len();
    let num_resources = resources.iter().filter(|&&r| r > 0).max().copied().unwrap_or(0);

    let mut m = String::new();
    m.push_str(&format!("int: num_tasks = {};\n", num_tasks));
    m.push_str(&format!("int: horizon = {};\n", horizon));
    m.push_str(&format!("int: num_resources = {};\n", num_resources));
    m.push_str(&format!("int: num_cooks = {};\n", num_cooks));
    m.push_str(&format!("int: num_recipes = {};\n", num_recipes));
    m.push_str(&format!("int: num_deps = {};\n", num_deps));

    m.push_str("array[1..num_tasks] of int: duration = [");
    for (i, d) in durations.iter().enumerate() {
        if i > 0 { m.push_str(", "); }
        m.push_str(&d.to_string());
    }
    m.push_str("];\n");

    m.push_str("array[1..num_tasks] of int: resource = [");
    for (i, r) in resources.iter().enumerate() {
        if i > 0 { m.push_str(", "); }
        m.push_str(&r.to_string());
    }
    m.push_str("];\n");

    m.push_str("array[1..num_tasks] of bool: needs_cook = [");
    for (i, n) in needs_cook_arr.iter().enumerate() {
        if i > 0 { m.push_str(", "); }
        m.push_str(if *n { "true" } else { "false" });
    }
    m.push_str("];\n");

    m.push_str("array[1..num_tasks] of int: recipe_of = [");
    for (i, r) in recipe_of.iter().enumerate() {
        if i > 0 { m.push_str(", "); }
        m.push_str(&(r + 1).to_string());
    }
    m.push_str("];\n");

    m.push_str("array[1..num_deps] of int: deps_from = [");
    for (i, v) in deps_from.iter().enumerate() {
        if i > 0 { m.push_str(", "); }
        m.push_str(&(v + 1).to_string());
    }
    m.push_str("];\n");

    m.push_str("array[1..num_deps] of int: deps_to = [");
    for (i, v) in deps_to.iter().enumerate() {
        if i > 0 { m.push_str(", "); }
        m.push_str(&(v + 1).to_string());
    }
    m.push_str("];\n");

    m.push_str("\
array[1..num_tasks] of var 0..horizon: start;
array[1..num_tasks] of var 0..num_cooks: cook;

constraint forall(i in 1..num_deps)(
  start[deps_to[i]] >= start[deps_from[i]] + duration[deps_from[i]]
);

constraint forall(i in 1..num_tasks, j in 1..num_tasks where i < j /\\ resource[i] > 0 /\\ resource[i] = resource[j])(
  start[i] + duration[i] <= start[j] \\/ start[j] + duration[j] <= start[i]
);

constraint forall(i in 1..num_tasks where needs_cook[i])(cook[i] > 0);
constraint forall(i in 1..num_tasks where not needs_cook[i])(cook[i] = 0);

constraint forall(i in 1..num_tasks, j in 1..num_tasks where i < j /\\ needs_cook[i] /\\ needs_cook[j] /\\ cook[i] = cook[j])(
  start[i] + duration[i] <= start[j] \\/ start[j] + duration[j] <= start[i]
);

array[1..num_recipes] of var 0..horizon: recipe_end;
constraint forall(r in 1..num_recipes)(
  recipe_end[r] = max([start[t] + duration[t] | t in 1..num_tasks where recipe_of[t] = r])
);

var 0..horizon: max_end = max(recipe_end);
var 0..horizon: min_recipe_end = min(recipe_end);

solve minimize max_end * (horizon + 1) + (max_end - min_recipe_end);

output [\"start = \", show(start), \";\\ncook = \", show(cook), \";\\n\"];");

    m
}

pub fn schedule(kitchen: &Kitchen, cooks: &[Cook], recipes: &[Recipe]) -> Plan {
    let num_recipes = recipes.len();

    let mut tasks = Vec::new();
    let mut id_to_idx: HashMap<String, usize> = HashMap::new();

    for (ri, recipe) in recipes.iter().enumerate() {
        for step in &recipe.steps {
            let tid = format!("{}:{}", recipe.name, step.id);
            let deps: Vec<String> = step.dependencies.iter()
                .map(|d| format!("{}:{}", recipe.name, d))
                .collect();
            let idx = tasks.len();
            id_to_idx.insert(tid.clone(), idx);
            tasks.push(TaskData {
                id: tid,
                description: step.description.clone(),
                duration_minutes: step.duration_minutes,
                resource_id: step.resource_id.clone(),
                dependencies: deps,
                recipe_idx: ri,
            });
        }
    }

    let mut resource_to_idx: HashMap<Option<String>, usize> = HashMap::new();
    resource_to_idx.insert(None, 0);
    for task in &tasks {
        let len = resource_to_idx.len();
        resource_to_idx.entry(task.resource_id.clone()).or_insert(len);
    }

    let mut deps_from = Vec::new();
    let mut deps_to = Vec::new();
    let mut encountered_deps = HashSet::new();
    for task in &tasks {
        let task_idx = id_to_idx[&task.id];
        for dep_id in &task.dependencies {
            if let Some(&dep_idx) = id_to_idx.get(dep_id) {
                if encountered_deps.insert((dep_idx, task_idx)) {
                    deps_from.push(dep_idx);
                    deps_to.push(task_idx);
                }
            }
        }
    }

    let num_cooks = cooks.len();
    let horizon: u32 = tasks.iter().map(|t| t.duration_minutes).sum();

    let durations: Vec<u32> = tasks.iter().map(|t| t.duration_minutes).collect();
    let resources: Vec<usize> = tasks.iter()
        .map(|t| resource_to_idx[&t.resource_id])
        .collect();
    let needs_cook_arr: Vec<bool> = tasks.iter()
        .map(|t| needs_cook(&t.resource_id, kitchen))
        .collect();
    let recipe_of: Vec<usize> = tasks.iter().map(|t| t.recipe_idx).collect();

    let model = build_model(
        &durations,
        &resources,
        &needs_cook_arr,
        &recipe_of,
        &deps_from,
        &deps_to,
        num_cooks,
        num_recipes,
        horizon,
    );

    let model_path = std::env::temp_dir().join(format!(
        "kitchen_planner_{}.mzn",
        std::process::id()
    ));
    let mut tmp = fs::File::create(&model_path).expect("failed to create temp file");
    write!(tmp, "{}", model).expect("failed to write model");
    drop(tmp);

    let output = Command::new("minizinc")
        .arg("--solver")
        .arg("gecode")
        .arg("--json-stream")
        .arg("--time-limit")
        .arg("10000")
        .arg(&model_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("failed to execute minizinc");

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    if !output.status.success() {
        eprintln!("minizinc error (exit code: {:?}):", output.status.code());
        eprintln!("stderr: {}", stderr);
        eprintln!("stdout: {}", stdout);
        eprintln!("model file: {}", model_path.display());
        std::process::exit(1);
    }

    let _ = fs::remove_file(&model_path);
    let mut last_solution: Option<(Vec<u32>, Vec<usize>)> = None;

    fn parse_array(s: &str) -> Option<Vec<i64>> {
        let s = s.trim();
        if !s.starts_with('[') || !s.ends_with(']') {
            return None;
        }
        let inner = &s[1..s.len() - 1];
        if inner.is_empty() {
            return Some(Vec::new());
        }
        inner.split(',')
            .map(|n| n.trim().parse::<i64>().ok())
            .collect()
    }

    for line in stdout.lines() {
        let parsed: serde_json::Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(_) => continue,
        };
        if parsed.get("type").and_then(|t| t.as_str()) != Some("solution") {
            continue;
        }
        let output_str = match parsed.get("output").and_then(|o| o.get("default")).and_then(|s| s.as_str()) {
            Some(s) => s,
            None => continue,
        };

        let mut start_vals: Option<Vec<u32>> = None;
        let mut cook_vals: Option<Vec<usize>> = None;

        for line in output_str.lines() {
            if let Some(arr_str) = line.strip_prefix("start = ").and_then(|s| s.strip_suffix(';')) {
                if let Some(v) = parse_array(arr_str) {
                    start_vals = Some(v.into_iter().map(|x| x as u32).collect());
                }
            }
            if let Some(arr_str) = line.strip_prefix("cook = ").and_then(|s| s.strip_suffix(';')) {
                if let Some(v) = parse_array(arr_str) {
                    cook_vals = Some(v.into_iter().map(|x| x as usize).collect());
                }
            }
        }

        if let (Some(start_vals), Some(cook_vals)) = (start_vals, cook_vals) {
            if start_vals.len() == tasks.len() && cook_vals.len() == tasks.len() {
                last_solution = Some((start_vals, cook_vals));
            }
        }
    }

    let (start_vals, cook_vals) = last_solution.expect("no solution found from minizinc");

    let plan_tasks: Vec<Task> = tasks.iter().enumerate().map(|(i, task)| {
        let cook_idx = cook_vals[i];
        let cook_name = if cook_idx > 0 && cook_idx <= cooks.len() {
            Some(cooks[cook_idx - 1].name.clone())
        } else {
            None
        };
        let deps_ids: Vec<String> = task.dependencies
            .iter()
            .filter(|d| id_to_idx.contains_key(d.as_str()))
            .cloned()
            .collect();

        Task {
            id: task.id.clone(),
            description: format!("{}: {}", task.id.split(':').next().unwrap_or(""), task.description),
            start_offset_minutes: start_vals[i],
            duration_minutes: task.duration_minutes,
            resource_id: task.resource_id.clone(),
            cook: cook_name,
            dependencies: deps_ids,
        }
    }).collect();

    Plan {
        start_time: "18:00".to_string(),
        tasks: plan_tasks,
    }
}

struct TaskData {
    id: String,
    description: String,
    duration_minutes: u32,
    resource_id: Option<String>,
    dependencies: Vec<String>,
    recipe_idx: usize,
}
