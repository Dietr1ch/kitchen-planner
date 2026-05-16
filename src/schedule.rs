use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::Write;
use std::process::{Command, Stdio};

use strum::IntoEnumIterator;

use crate::models::cook::{Cook, SkillLevel, duration_for_skill};
use crate::models::kitchen::Kitchen;
use crate::models::plan::{Plan, Task};
use crate::models::recipe::Recipe;

#[derive(Debug, thiserror::Error)]
pub enum ScheduleError {
	#[error("failed to create or write model file")]
	IO(#[from] std::io::Error),
	#[error("solver failed: {0}")]
	SolverFailure(String),
	#[error("no solution found from solver")]
	NoSolution,
}

#[allow(clippy::too_many_arguments)]
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
	eff_duration: &[Vec<u32>],
	num_skills: usize,
	cook_skill_level: &[Vec<usize>],
	required_skill: &[usize],
	min_level: &[usize],
	preheat_indices: &[usize],
	preheat_bake_indices: &[usize],
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
	m.push_str(&format!("int: num_skills = {};\n", num_skills));

	m.push_str("array[1..num_tasks] of int: duration = [");
	for (i, d) in durations.iter().enumerate() {
		if i > 0 {
			m.push_str(", ");
		}
		m.push_str(&d.to_string());
	}
	m.push_str("];\n");

	let eff_flat: Vec<String> = eff_duration
		.iter()
		.flat_map(|row| row.iter())
		.map(|v| v.to_string())
		.collect();
	m.push_str(&format!(
		"array[0..num_cooks, 1..num_tasks] of int: eff_duration = array2d(0..num_cooks, 1..num_tasks, [{}]);\n",
		eff_flat.join(", "),
	));

	m.push_str("array[1..num_tasks] of var 0..horizon: actual_duration;\n");
	m.push_str("constraint forall(t in 1..num_tasks)(\n");
	m.push_str(
		"  actual_duration[t] = if needs_cook[t] then eff_duration[cook[t], t] else duration[t] endif\n",
	);
	m.push_str(");\n");

	m.push_str("array[1..num_equipment] of int: equip_kind = [");
	for (i, k) in equip_kind.iter().enumerate() {
		if i > 0 {
			m.push_str(", ");
		}
		m.push_str(&k.to_string());
	}
	m.push_str("];\n");

	m.push_str("array[1..num_tasks] of int: task_kind = [");
	for (i, k) in task_kind.iter().enumerate() {
		if i > 0 {
			m.push_str(", ");
		}
		m.push_str(&k.to_string());
	}
	m.push_str("];\n");

	m.push_str(&format!(
		"array[1..num_kinds] of int: kind_start = [{}];\n",
		kind_start[1..]
			.iter()
			.map(|v| v.to_string())
			.collect::<Vec<_>>()
			.join(", ")
	));
	m.push_str(&format!(
		"array[1..num_kinds] of int: kind_end = [{}];\n",
		kind_end[1..]
			.iter()
			.map(|v| v.to_string())
			.collect::<Vec<_>>()
			.join(", ")
	));

	m.push_str("array[1..num_tasks] of bool: needs_cook = [");
	for (i, n) in needs_cook_arr.iter().enumerate() {
		if i > 0 {
			m.push_str(", ");
		}
		m.push_str(if *n { "true" } else { "false" });
	}
	m.push_str("];\n");

	m.push_str("array[1..num_tasks] of int: recipe_of = [");
	for (i, r) in recipe_of.iter().enumerate() {
		if i > 0 {
			m.push_str(", ");
		}
		m.push_str(&(r + 1).to_string());
	}
	m.push_str("];\n");

	m.push_str(&format!("int: num_preheats = {};\n", preheat_indices.len()));
	m.push_str("array[1..num_preheats] of int: preheat_tasks = [");
	for (i, &idx) in preheat_indices.iter().enumerate() {
		if i > 0 {
			m.push_str(", ");
		}
		m.push_str(&(idx + 1).to_string());
	}
	m.push_str("];\n");
	m.push_str("array[1..num_preheats] of int: preheat_bakes = [");
	for (i, &idx) in preheat_bake_indices.iter().enumerate() {
		if i > 0 {
			m.push_str(", ");
		}
		m.push_str(&(idx + 1).to_string());
	}
	m.push_str("];\n");

	m.push_str("array[1..num_deps] of int: deps_from = [");
	for (i, v) in deps_from.iter().enumerate() {
		if i > 0 {
			m.push_str(", ");
		}
		m.push_str(&(v + 1).to_string());
	}
	m.push_str("];\n");

	m.push_str("array[1..num_deps] of int: deps_to = [");
	for (i, v) in deps_to.iter().enumerate() {
		if i > 0 {
			m.push_str(", ");
		}
		m.push_str(&(v + 1).to_string());
	}
	m.push_str("];\n");

	{
		let flat: Vec<String> = cook_skill_level
			.iter()
			.flat_map(|row| row[1..].iter())
			.map(|v| v.to_string())
			.collect();
		m.push_str(&format!(
			"array[0..num_cooks, 1..num_skills] of int: cook_skill_level = array2d(0..num_cooks, 1..num_skills, [{}]);\n",
			flat.join(", "),
		));
	}

	m.push_str("array[1..num_tasks] of int: required_skill = [");
	for (i, &s) in required_skill.iter().enumerate() {
		if i > 0 {
			m.push_str(", ");
		}
		m.push_str(&s.to_string());
	}
	m.push_str("];\n");

	m.push_str("array[1..num_tasks] of int: min_level = [");
	for (i, &l) in min_level.iter().enumerate() {
		if i > 0 {
			m.push_str(", ");
		}
		m.push_str(&l.to_string());
	}
	m.push_str("];\n");

	m.push_str("\
array[1..num_tasks] of var 0..horizon: start;
array[1..num_tasks] of var 0..num_cooks: cook;
array[1..num_tasks] of var 0..num_equipment: assign;

constraint forall(i in 1..num_deps)(
  start[deps_to[i]] >= start[deps_from[i]] + actual_duration[deps_from[i]]
);

constraint forall(i in 1..num_tasks, j in 1..num_tasks where i < j /\\ assign[i] > 0 /\\ assign[i] = assign[j])(
  start[i] + actual_duration[i] <= start[j] \\/ start[j] + actual_duration[j] <= start[i]
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
  start[i] + actual_duration[i] <= start[j] \\/ start[j] + actual_duration[j] <= start[i]
);

constraint forall(t in 1..num_tasks where required_skill[t] > 0)(
  cook_skill_level[cook[t], required_skill[t]] >= min_level[t]
);

array[1..num_recipes] of var 0..horizon: recipe_end;
constraint forall(r in 1..num_recipes)(
  recipe_end[r] = max([start[t] + actual_duration[t] | t in 1..num_tasks where recipe_of[t] = r])
);

var 0..horizon: max_end = max(recipe_end);
var 0..horizon: min_recipe_end = min(recipe_end);

var 0..horizon: total_waste = if num_preheats > 0 then sum(p in 1..num_preheats)(start[preheat_bakes[p]] - (start[preheat_tasks[p]] + actual_duration[preheat_tasks[p]])) else 0 endif;

solve minimize max_end * (horizon + 1) * (1 + num_preheats) + (max_end - min_recipe_end) * (1 + num_preheats) + total_waste;

output [\"start = \", show(start), \";\\ncook = \", show(cook), \";\\nassign = \", show(assign), \";\\n\"];");

	m
}

pub fn schedule(
	kitchen: &Kitchen,
	cooks: &[Cook],
	recipes: &[Recipe],
) -> Result<Plan, ScheduleError> {
	let num_recipes = recipes.len();

	let mut tasks = Vec::new();
	let mut id_to_idx: HashMap<String, usize> = HashMap::new();

	for (ri, recipe) in recipes.iter().enumerate() {
		for step in &recipe.steps {
			let tid = format!("{}:{}", recipe.name, step.id);
			let deps: Vec<String> = step
				.dependencies
				.iter()
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
				needs_cook: step.needs_cook,
				duration_by_skill: step.duration_by_skill.clone(),
				skill: step.skill.clone(),
				min_skill_level: step.min_skill_level,
				temperature_celsius: step.temperature_celsius,
			});
		}
	}

	// Inject pre-heat tasks for steps requiring a specific temperature.
	// Duration is computed from the fastest equipment matching the required kind.
	// See README for details on this assumption.
	// Pre-heat tasks depend on the grandparent steps of the bake step
	// (dependencies of the bake's direct dependencies) so the oven doesn't
	// heat far before the ingredients are ready.
	let mut preheat_insertions: Vec<(usize, String, u32, String, u16)> = Vec::new();
	for (i, task) in tasks.iter().enumerate() {
		if let Some(temp) = task.temperature_celsius
			&& let Some(ref kind) = task.resource_kind
		{
			let min_rate = kitchen
				.equipment
				.iter()
				.filter(|e| e.kind == *kind)
				.map(|e| e.preheat_rate_minutes_per_celsius)
				.fold(f64::INFINITY, f64::min);
			if min_rate.is_finite() {
				let delta = temp as f64 - kitchen.ambient_temperature_celsius;
				let duration = (min_rate * delta).round() as u32;
				let preheat_id = format!("{}.preheat", task.id);
				preheat_insertions.push((i, preheat_id, duration, kind.clone(), temp));
			}
		}
	}
	let mut preheat_pairs: Vec<(usize, usize)> = Vec::new();
	for (bake_idx, preheat_id, duration, kind, temp) in preheat_insertions {
		let recipe_idx = tasks[bake_idx].recipe_idx;
		let preheat_task = TaskData {
			id: preheat_id.clone(),
			description: format!("Pre-heat {} to {}°C", kind, temp),
			duration_minutes: duration,
			resource_kind: Some(kind),
			dependencies: Vec::new(),
			recipe_idx,
			needs_cook: false,
			duration_by_skill: None,
			skill: None,
			min_skill_level: None,
			temperature_celsius: None,
		};
		let idx = tasks.len();
		id_to_idx.insert(preheat_id.clone(), idx);
		tasks.push(preheat_task);
		tasks[bake_idx].dependencies.push(preheat_id);
		preheat_pairs.push((idx, bake_idx));
	}

	let equipment: Vec<EquipInfo> = kitchen
		.equipment
		.iter()
		.map(|e| EquipInfo {
			name: e.name.clone(),
			kind: e.kind.clone(),
		})
		.collect();
	let num_equipment = equipment.len();

	let mut kind_to_int: HashMap<&str, usize> = HashMap::new();
	for eq in &equipment {
		let len = kind_to_int.len();
		kind_to_int.entry(eq.kind.as_str()).or_insert(len + 1);
	}
	let num_kinds = kind_to_int.len();

	let equip_kind: Vec<usize> = equipment
		.iter()
		.map(|eq| kind_to_int[eq.kind.as_str()])
		.collect();

	let mut kind_start = vec![num_equipment + 1; num_kinds + 1];
	let mut kind_end = vec![0usize; num_kinds + 1];
	for (i, &k) in equip_kind.iter().enumerate() {
		let idx = i + 1;
		if idx < kind_start[k] {
			kind_start[k] = idx;
		}
		if idx > kind_end[k] {
			kind_end[k] = idx;
		}
	}

	let task_kind: Vec<usize> = tasks
		.iter()
		.map(|t| {
			t.resource_kind
				.as_deref()
				.and_then(|k| kind_to_int.get(k).copied())
				.unwrap_or(0)
		})
		.collect();

	let mut deps_from = Vec::new();
	let mut deps_to = Vec::new();
	let mut encountered_deps = HashSet::new();
	for task in &tasks {
		let task_idx = id_to_idx[&task.id];
		for dep_id in &task.dependencies {
			if let Some(&dep_idx) = id_to_idx.get(dep_id)
				&& encountered_deps.insert((dep_idx, task_idx))
			{
				deps_from.push(dep_idx);
				deps_to.push(task_idx);
			}
		}
	}

	let num_cooks = cooks.len();
	let horizon: u32 = tasks.iter().map(|t| t.duration_minutes).sum();

	let durations: Vec<u32> = tasks.iter().map(|t| t.duration_minutes).collect();
	let needs_cook_arr: Vec<bool> = tasks.iter().map(|t| t.needs_cook).collect();
	let recipe_of: Vec<usize> = tasks.iter().map(|t| t.recipe_idx).collect();

	// Collect unique skill names referenced by recipe steps
	let mut skill_to_idx: HashMap<&str, usize> = HashMap::new();
	for task in &tasks {
		if let Some(ref skill_name) = task.skill {
			let len = skill_to_idx.len();
			skill_to_idx.entry(skill_name).or_insert(len + 1);
		}
	}
	let num_skills = skill_to_idx.len();

	// Build cook-skill matrix: cook_skill_level[c][s] = numeric level (0..=4)
	let mut cook_skill_level = vec![vec![0usize; num_skills + 1]; num_cooks + 1];
	for (ci, cook) in cooks.iter().enumerate() {
		let c = ci + 1;
		for (skill_name, level) in &cook.skills {
			if let Some(&si) = skill_to_idx.get(skill_name.as_str()) {
				cook_skill_level[c][si] = *level as u8 as usize;
			}
		}
	}

	// Per-task skill arrays
	let required_skill: Vec<usize> = tasks
		.iter()
		.map(|t| {
			t.skill
				.as_deref()
				.and_then(|s| skill_to_idx.get(s).copied())
				.unwrap_or(0)
		})
		.collect();

	let min_level: Vec<usize> = tasks
		.iter()
		.map(|t| t.min_skill_level.map(|l| l as u8 as usize).unwrap_or(0))
		.collect();

	// Pre-compute effective durations per (cook, task) pair
	let num_tasks = tasks.len();
	let mut eff_duration = vec![vec![0u32; num_tasks]; num_cooks + 1];
	for c in 0..=num_cooks {
		for (t, task) in tasks.iter().enumerate() {
			if c == 0 {
				eff_duration[c][t] = task.duration_minutes;
			} else if let Some(ref map) = task.duration_by_skill {
				let si = required_skill[t];
				let level = SkillLevel::iter()
					.nth(cook_skill_level[c][si])
					.expect("valid skill level index");
				eff_duration[c][t] =
					duration_for_skill(map, level).unwrap_or(task.duration_minutes);
			} else {
				eff_duration[c][t] = task.duration_minutes;
			}
		}
	}

	let preheat_indices: Vec<usize> = preheat_pairs.iter().map(|&(p, _)| p).collect();
	let preheat_bake_indices: Vec<usize> = preheat_pairs.iter().map(|&(_, b)| b).collect();
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
		&eff_duration,
		num_skills,
		&cook_skill_level,
		&required_skill,
		&min_level,
		&preheat_indices,
		&preheat_bake_indices,
	);

	let model_path =
		std::env::temp_dir().join(format!("kitchen_planner_{}.mzn", std::process::id()));
	let mut tmp = fs::File::create(&model_path)?;
	write!(tmp, "{}", model)?;
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
		.output()?;

	let stderr = String::from_utf8_lossy(&output.stderr);
	let stdout = String::from_utf8_lossy(&output.stdout);

	if !output.status.success() {
		let mut msg = format!("minizinc exited with code {:?}", output.status.code(),);
		if !stderr.is_empty() {
			msg.push_str(&format!("\nstderr: {}", stderr));
		}
		return Err(ScheduleError::SolverFailure(msg));
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
		inner
			.split(',')
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
		let output_str = match parsed
			.get("output")
			.and_then(|o| o.get("default"))
			.and_then(|s| s.as_str())
		{
			Some(s) => s,
			None => continue,
		};

		let mut start_vals: Option<Vec<u32>> = None;
		let mut cook_vals: Option<Vec<usize>> = None;
		let mut assign_vals: Option<Vec<usize>> = None;

		for line in output_str.lines() {
			if let Some(arr_str) = line
				.strip_prefix("start = ")
				.and_then(|s| s.strip_suffix(';'))
				&& let Some(v) = parse_array_i64(arr_str)
			{
				start_vals = Some(v.into_iter().map(|x| x as u32).collect());
			}
			if let Some(arr_str) = line
				.strip_prefix("cook = ")
				.and_then(|s| s.strip_suffix(';'))
				&& let Some(v) = parse_array_i64(arr_str)
			{
				cook_vals = Some(v.into_iter().map(|x| x as usize).collect());
			}
			if let Some(arr_str) = line
				.strip_prefix("assign = ")
				.and_then(|s| s.strip_suffix(';'))
				&& let Some(v) = parse_array_i64(arr_str)
			{
				assign_vals = Some(v.into_iter().map(|x| x as usize).collect());
			}
		}

		if let (Some(start_vals), Some(cook_vals), Some(assign_vals)) =
			(start_vals, cook_vals, assign_vals)
			&& start_vals.len() == tasks.len()
			&& cook_vals.len() == tasks.len()
			&& assign_vals.len() == tasks.len()
		{
			last_solution = Some((start_vals, cook_vals, assign_vals));
		}
	}

	let (start_vals, cook_vals, assign_vals) = last_solution.ok_or(ScheduleError::NoSolution)?;

	let plan_tasks: Vec<Task> = tasks
		.iter()
		.enumerate()
		.map(|(i, task)| {
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
			let deps_ids: Vec<String> = task
				.dependencies
				.iter()
				.filter(|d| id_to_idx.contains_key(d.as_str()))
				.cloned()
				.collect();

			let actual_dur = if needs_cook_arr[i] {
				eff_duration[cook_vals[i]][i]
			} else {
				durations[i]
			};

			Task {
				id: task.id.clone(),
				dish: task.id.split(':').next().unwrap_or("").to_string(),
				description: task.description.clone(),
				start_offset_minutes: start_vals[i],
				duration_minutes: actual_dur,
				resource_id: assigned_resource,
				resource_kind: task.resource_kind.clone(),
				cook: cook_name,
				dependencies: deps_ids,
			}
		})
		.collect();

	Ok(Plan { tasks: plan_tasks })
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
	needs_cook: bool,
	duration_by_skill: Option<HashMap<SkillLevel, u32>>,
	skill: Option<String>,
	min_skill_level: Option<SkillLevel>,
	temperature_celsius: Option<u16>,
}
