use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::Write;
use std::process::{Command, Stdio};

use crate::models::cook::Cook;
use crate::models::kitchen::Kitchen;
use crate::models::plan::{Plan, Task};
use crate::models::recipe::Recipe;

fn needs_cook(resource_kind: &Option<String>) -> bool {
    resource_kind.as_deref().map_or(false, |k| k != "oven")
}

fn build_model(
    durations: &[u32],
    needs_cook_arr: &[bool],
    recipe_of: &[usize],
    deps_from: &[usize],
    deps_to: &[usize],
    num_cooks: usize,
    num_recipes: usize,
    horizon: u32,
    num_equipment: usize,
    num_kinds: usize,
    equip_kind: &[usize],
    task_kind: &[usize],
    kind_start: &[usize],
    kind_end: &[usize],
) -> String {
    let num_tasks = durations.len();
    let num_deps = deps_from.len();

    let mut m = String::new();
    m.push_str(&format!("int: num_tasks = {};\n", num_tasks));
    m.push_str(&format!("int: horizon = {};\n", horizon));
    m.push_str(&format!("int: num_cooks = {};\n", num_cooks));
    m.push_str(&format!("int: num_recipes = {};\n", num_recipes));
    m.push_str(&format!("int: num_deps = {};\n", num_deps));
    m.push_str(&format!("int: num_equipment = {};\n", num_equipment));
    m.push_str(&format!("int: num_kinds = {};\n", num_kinds));

    m.push_str("array[1..num_tasks] of int: duration = [");
    for (i, d) in durations.iter().enumerate() {
        if i > 0 { m.push_str(", "); }
        m.push_str(&d.to_string());
    }
    m.push_str("];\n");

    m.push_str("array[1..num_equipment] of int: equip_kind = [");
    for (i, k) in equip_kind.iter().enumerate() {
        if i > 0 { m.push_str(", "); }
        m.push_str(&k.to_string());
    }
    m.push_str("];\n");

    m.push_str("array[1..num_tasks] of int: task_kind = [");
    for (i, k) in task_kind.iter().enumerate() {
        if i > 0 { m.push_str(", "); }
        m.push_str(&k.to_string());
    }
    m.push_str("];\n");

    m.push_str("array[1..num_kinds] of int: kind_start = [");
    for i in 1..kind_start.len() {
        if i > 1 { m.push_str(", "); }
        m.push_str(&kind_start[i].to_string());
    }
    m.push_str("];\n");

    m.push_str("array[1..num_kinds] of int: kind_end = [");
    for i in 1..kind_end.len() {
        if i > 1 { m.push_str(", "); }
        m.push_str(&kind_end[i].to_string());
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
array[1..num_tasks] of var 0..num_equipment: assign;

constraint forall(i in 1..num_deps)(
  start[deps_to[i]] >= start[deps_from[i]] + duration[deps_from[i]]
);

constraint forall(i in 1..num_tasks, j in 1..num_tasks where i < j /\\ assign[i] > 0 /\\ assign[i] = assign[j])(
  start[i] + duration[i] <= start[j] \\/ start[j] + duration[j] <= start[i]
);

constraint forall(t in 1..num_tasks where task_kind[t] > 0)(
  assign[t] >= kind_start[task_kind[t]] /\\ assign[t] <= kind_end[task_kind[t]]
);
constraint forall(t in 1..num_tasks where task_kind[t] == 0)(
  assign[t] == 0
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

output [\"start = \", show(start), \";\\ncook = \", show(cook), \";\\nassign = \", show(assign), \";\\n\"];");

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
                resource_kind: step.resource_kind.clone(),
                dependencies: deps,
                recipe_idx: ri,
            });
        }
    }

    let equipment: Vec<EquipInfo> = kitchen.equipment.iter()
        .map(|e| EquipInfo { name: e.name.clone(), kind: e.kind.clone() })
        .collect();
    let num_equipment = equipment.len();

    let mut kind_to_int: HashMap<&str, usize> = HashMap::new();
    for eq in &equipment {
        let len = kind_to_int.len();
        kind_to_int.entry(eq.kind.as_str()).or_insert(len + 1);
    }
    let num_kinds = kind_to_int.len();

    let equip_kind: Vec<usize> = equipment.iter()
        .map(|eq| kind_to_int[eq.kind.as_str()])
        .collect();

    let mut kind_start = vec![num_equipment + 1; num_kinds + 1];
    let mut kind_end = vec![0usize; num_kinds + 1];
    for (i, &k) in equip_kind.iter().enumerate() {
        let idx = i + 1;
        if idx < kind_start[k] { kind_start[k] = idx; }
        if idx > kind_end[k] { kind_end[k] = idx; }
    }

    let task_kind: Vec<usize> = tasks.iter()
        .map(|t| t.resource_kind.as_deref().and_then(|k| kind_to_int.get(k).copied()).unwrap_or(0))
        .collect();

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
    let needs_cook_arr: Vec<bool> = tasks.iter()
        .map(|t| needs_cook(&t.resource_kind))
        .collect();
    let recipe_of: Vec<usize> = tasks.iter().map(|t| t.recipe_idx).collect();

    let model = build_model(
        &durations,
        &needs_cook_arr,
        &recipe_of,
        &deps_from,
        &deps_to,
        num_cooks,
        num_recipes,
        horizon,
        num_equipment,
        num_kinds,
        &equip_kind,
        &task_kind,
        &kind_start,
        &kind_end,
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
    let mut last_solution: Option<(Vec<u32>, Vec<usize>, Vec<usize>)> = None;

    fn parse_array_i64(s: &str) -> Option<Vec<i64>> {
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
        let mut assign_vals: Option<Vec<usize>> = None;

        for line in output_str.lines() {
            if let Some(arr_str) = line.strip_prefix("start = ").and_then(|s| s.strip_suffix(';')) {
                if let Some(v) = parse_array_i64(arr_str) {
                    start_vals = Some(v.into_iter().map(|x| x as u32).collect());
                }
            }
            if let Some(arr_str) = line.strip_prefix("cook = ").and_then(|s| s.strip_suffix(';')) {
                if let Some(v) = parse_array_i64(arr_str) {
                    cook_vals = Some(v.into_iter().map(|x| x as usize).collect());
                }
            }
            if let Some(arr_str) = line.strip_prefix("assign = ").and_then(|s| s.strip_suffix(';')) {
                if let Some(v) = parse_array_i64(arr_str) {
                    assign_vals = Some(v.into_iter().map(|x| x as usize).collect());
                }
            }
        }

        if let (Some(start_vals), Some(cook_vals), Some(assign_vals)) = (start_vals, cook_vals, assign_vals) {
            if start_vals.len() == tasks.len() && cook_vals.len() == tasks.len() && assign_vals.len() == tasks.len() {
                last_solution = Some((start_vals, cook_vals, assign_vals));
            }
        }
    }

    let (start_vals, cook_vals, assign_vals) = last_solution.expect("no solution found from minizinc");

    let plan_tasks: Vec<Task> = tasks.iter().enumerate().map(|(i, task)| {
        let cook_idx = cook_vals[i];
        let cook_name = if cook_idx > 0 && cook_idx <= cooks.len() {
            Some(cooks[cook_idx - 1].name.clone())
        } else {
            None
        };
        let assign_idx = assign_vals[i];
        let assigned_resource = if assign_idx > 0 && assign_idx <= equipment.len() {
            Some(equipment[assign_idx - 1].name.clone())
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
            dish: task.id.split(':').next().unwrap_or("").to_string(),
            description: task.description.clone(),
            start_offset_minutes: start_vals[i],
            duration_minutes: task.duration_minutes,
            resource_id: assigned_resource,
            resource_kind: task.resource_kind.clone(),
            cook: cook_name,
            dependencies: deps_ids,
        }
    }).collect();

    Plan {
        start_time: "18:00".to_string(),
        tasks: plan_tasks,
    }
}

struct EquipInfo {
    name: String,
    kind: String,
}

struct TaskData {
    id: String,
    description: String,
    duration_minutes: u32,
    resource_kind: Option<String>,
    dependencies: Vec<String>,
    recipe_idx: usize,
}
