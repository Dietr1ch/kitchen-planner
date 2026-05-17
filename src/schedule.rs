use std::collections::{HashMap, HashSet};
use std::io::Write;
use std::process::{Command, Stdio};

use strum::IntoEnumIterator;

use crate::models::cook::{Cook, SkillLevel, duration_for_skill};
use crate::models::kitchen::Kitchen;
use crate::models::plan::{Plan, Task};
use crate::models::recipe::Recipe;

use googletest::prelude::*;

#[derive(Debug, thiserror::Error)]
pub enum ScheduleError {
	#[error("failed to create or write model file")]
	IO(#[from] std::io::Error),
	#[error("solver failed: {0}")]
	SolverFailure(String),
	#[error("Unfeasible problem: {0}")]
	Unfeasible(String),
	#[error("no solution found from solver")]
	NoSolution,
}

const MODEL: &str = include_str!("model.mzn");

struct TaskData {
	id: String,
	description: String,
	duration_minutes: u32,
	resource_kinds: Vec<String>,
	dependencies: Vec<String>,
	recipe_idx: usize,
	needs_cook: bool,
	duration_by_skill: Option<HashMap<SkillLevel, u32>>,
	skill: Option<String>,
	min_skill_level: Option<SkillLevel>,
	temperature_celsius: Option<u16>,
}

struct DznWriter {
	content: String,
}

impl DznWriter {
	fn new() -> Self {
		DznWriter { content: String::new() }
	}

	fn param(&mut self, name: &str, value: impl std::fmt::Display) {
		self.content.push_str(&format!("{} = {};\n", name, value));
	}

	fn int_array(&mut self, name: &str, _lo: usize, _hi: usize, values: &[i64]) {
		self.content.push_str(&format!(
			"{} = [{}];\n",
			name,
			values
				.iter()
				.map(|v| v.to_string())
				.collect::<Vec<_>>()
				.join(", ")
		));
	}

	fn bool_array(&mut self, name: &str, _lo: usize, _hi: usize, values: &[bool]) {
		self.content.push_str(&format!(
			"{} = [{}];\n",
			name,
			values
				.iter()
				.map(|v| if *v { "true" } else { "false" })
				.collect::<Vec<_>>()
				.join(", ")
		));
	}

	fn int_array2d(
		&mut self,
		name: &str,
		rlo: usize,
		rhi: usize,
		clo: usize,
		chi: usize,
		values: &[i64],
	) {
		self.content.push_str(&format!(
			"{} = array2d({}..{}, {}..{}, [{}]);\n",
			name,
			rlo,
			rhi,
			clo,
			chi,
			values
				.iter()
				.map(|v| v.to_string())
				.collect::<Vec<_>>()
				.join(", ")
		));
	}
}

pub fn schedule(
	kitchen: &Kitchen,
	cooks: &[Cook],
	recipes: &[Recipe],
) -> Result<Plan, ScheduleError> {
	let (tasks, preheat_pairs) = expand_tasks(recipes, kitchen);
	let num_tasks = tasks.len();
	if num_tasks == 0 {
		return Err(ScheduleError::Unfeasible("no tasks to schedule".into()));
	}

	let id_to_idx: HashMap<String, usize> = tasks
		.iter()
		.enumerate()
		.map(|(i, t)| (t.id.clone(), i))
		.collect();

	let equipment: Vec<EquipInfo> = kitchen
		.equipment
		.iter()
		.map(|e| EquipInfo { name: e.name.clone(), kind: e.kind.clone() })
		.collect();
	let num_equipment = equipment.len();

	let (kind_to_idx, equip_kind) = build_equip_kind_mapping(&equipment);
	let num_kinds = kind_to_idx.len();

	let (kind_start, kind_end) = build_kind_ranges(&equip_kind, num_kinds);

	let task_kinds: Vec<Vec<usize>> = build_task_kinds(&tasks, &kind_to_idx);
	let max_resources = task_kinds.iter().map(|v| v.len()).max().unwrap_or(0);
	let task_kinds_flat: Vec<i64> = task_kinds
		.iter()
		.flat_map(|v| {
			let mut padded = v.clone();
			padded.resize(max_resources, 0);
			padded.into_iter().map(|x| x as i64)
		})
		.collect();

	let (deps_from, deps_to) = build_dependencies(&tasks, &id_to_idx);

	let durations: Vec<i64> = tasks.iter().map(|t| t.duration_minutes as i64).collect();
	let needs_cook_arr: Vec<bool> = tasks.iter().map(|t| t.needs_cook).collect();
	let recipe_of: Vec<i64> = tasks.iter().map(|t| (t.recipe_idx + 1) as i64).collect();

	let num_cooks = cooks.len();
	let num_recipes = recipes.len();
	let horizon: u32 = tasks.iter().map(|t| t.duration_minutes).sum::<u32>() * 2;

	let (_skill_to_idx, num_skills, required_skill, min_level, cook_skill_level) =
		build_skill_data(&tasks, cooks);

	let eff_duration = compute_effective_durations(&tasks, cooks, &required_skill, &cook_skill_level);

	let preheat_tasks: Vec<i64> =
		preheat_pairs.iter().map(|p| (p.preheat_idx + 1) as i64).collect();
	let preheat_bakes: Vec<i64> =
		preheat_pairs.iter().map(|p| (p.bake_idx + 1) as i64).collect();
	let num_preheats = preheat_pairs.len();

	let dzn = build_dzn(
		num_tasks,
		horizon,
		num_cooks,
		num_recipes,
		deps_from.len(),
		num_equipment,
		num_kinds,
		max_resources,
		num_skills,
		num_preheats,
		&durations,
		&needs_cook_arr,
		&recipe_of,
		&deps_from,
		&deps_to,
		&equip_kind,
		&task_kinds_flat,
		&kind_start,
		&kind_end,
		&eff_duration,
		&cook_skill_level,
		&required_skill,
		&min_level,
		&preheat_tasks,
		&preheat_bakes,
	);

	let model_input = format!("{}\n{}", MODEL, dzn);
	let solution = run_solver(&model_input, &tasks)?;

	parse_solution(&solution, &tasks, &id_to_idx, cooks, &equipment, max_resources, &durations, &eff_duration, &needs_cook_arr)
}

struct EquipInfo {
	name: String,
	kind: String,
}

fn build_equip_kind_mapping(equipment: &[EquipInfo]) -> (HashMap<String, usize>, Vec<i64>) {
	let mut kind_to_idx: HashMap<String, usize> = HashMap::new();
	for eq in equipment {
		let len = kind_to_idx.len();
		kind_to_idx.entry(eq.kind.clone()).or_insert(len + 1);
	}
	let num_kinds = kind_to_idx.len();
	let equip_kind: Vec<i64> = equipment
		.iter()
		.map(|eq| {
			let k = kind_to_idx[&eq.kind] as i64;
			debug_assert!(k >= 1 && k as usize <= num_kinds, "equipment kind {} out of range 1..{}", k, num_kinds);
			k
		})
		.collect();
	(kind_to_idx, equip_kind)
}

fn build_kind_ranges(equip_kind: &[i64], num_kinds: usize) -> (Vec<i64>, Vec<i64>) {
	let mut kind_start = vec![i64::MAX; num_kinds];
	let mut kind_end = vec![i64::MIN; num_kinds];
	for (i, &k) in equip_kind.iter().enumerate() {
		let kind = (k - 1) as usize;
		debug_assert!(kind < num_kinds, "kind index {} out of range 0..{}", kind, num_kinds);
		let pos = (i + 1) as i64;
		if pos < kind_start[kind] {
			kind_start[kind] = pos;
		}
		if pos > kind_end[kind] {
			kind_end[kind] = pos;
		}
	}
	for k in 0..num_kinds {
		if kind_start[k] == i64::MAX {
			kind_start[k] = 0;
		}
		if kind_end[k] == i64::MIN {
			kind_end[k] = 0;
		}
		debug_assert!(kind_start[k] <= kind_end[k], "kind {} has empty range ({}..{})", k, kind_start[k], kind_end[k]);
	}
	(kind_start, kind_end)
}

fn build_task_kinds(tasks: &[TaskData], kind_to_idx: &HashMap<String, usize>) -> Vec<Vec<usize>> {
	tasks
		.iter()
		.map(|t| {
			t.resource_kinds
				.iter()
				.map(|k| kind_to_idx.get(k.as_str()).copied().unwrap_or(0))
				.collect()
		})
		.collect()
}

fn build_dependencies(
	tasks: &[TaskData],
	id_to_idx: &HashMap<String, usize>,
) -> (Vec<i64>, Vec<i64>) {
	let num_tasks = tasks.len();
	let mut deps_from = Vec::new();
	let mut deps_to = Vec::new();
	let mut seen = HashSet::new();
	for task in tasks {
		let task_idx = id_to_idx[&task.id];
		debug_assert!(task_idx < num_tasks);
		for dep_id in &task.dependencies {
			if let Some(&dep_idx) = id_to_idx.get(dep_id)
				&& seen.insert((dep_idx, task_idx))
			{
				debug_assert!(dep_idx < num_tasks);
				deps_from.push((dep_idx + 1) as i64);
				deps_to.push((task_idx + 1) as i64);
			}
		}
	}
	debug_assert!(deps_from.iter().all(|&f| f >= 1 && f as usize <= num_tasks));
	debug_assert!(deps_to.iter().all(|&t| t >= 1 && t as usize <= num_tasks));
	(deps_from, deps_to)
}

#[allow(clippy::type_complexity)]
fn build_skill_data(
	tasks: &[TaskData],
	cooks: &[Cook],
) -> (HashMap<String, usize>, usize, Vec<i64>, Vec<i64>, Vec<Vec<i64>>) {
	let mut skill_to_idx: HashMap<String, usize> = HashMap::new();
	for task in tasks {
		if let Some(ref skill_name) = task.skill {
			let len = skill_to_idx.len();
			skill_to_idx.entry(skill_name.clone()).or_insert(len + 1);
		}
	}
	let num_skills = skill_to_idx.len();

	let required_skill: Vec<i64> = tasks
		.iter()
		.map(|t| {
			t.skill
				.as_deref()
				.and_then(|s| skill_to_idx.get(s).copied())
				.map(|v| v as i64)
				.unwrap_or(-1)
		})
		.collect();

	for (t, &sk) in required_skill.iter().enumerate() {
		debug_assert!(sk == -1 || (sk >= 1 && sk as usize <= num_skills),
			"task {} skill index {} out of range 1..{}", t, sk, num_skills);
	}

	let min_level: Vec<i64> = tasks
		.iter()
		.map(|t| t.min_skill_level.map(|l| l as u8 as i64).unwrap_or(0))
		.collect();

	debug_assert!(min_level.iter().all(|&l| l >= 0 && l <= 4), "min_level out of range 0..4");

	let num_cooks = cooks.len();
	let mut cook_skill_level = vec![vec![0i64; num_skills.max(1)]; num_cooks + 1];
	for (ci, cook) in cooks.iter().enumerate() {
		let c = ci + 1;
		for (skill_name, level) in &cook.skills {
			if let Some(&si) = skill_to_idx.get(skill_name) {
				debug_assert!(si >= 1 && si <= num_skills, "skill index {} out of range 1..{}", si, num_skills);
				cook_skill_level[c][si - 1] = *level as u8 as i64;
			}
		}
	}

	debug_assert!(cook_skill_level.len() == num_cooks + 1);
	debug_assert!(cook_skill_level.iter().all(|row| row.len() == num_skills.max(1)));

	(skill_to_idx, num_skills, required_skill, min_level, cook_skill_level)
}

fn compute_effective_durations(
	tasks: &[TaskData],
	cooks: &[Cook],
	required_skill: &[i64],
	cook_skill_level: &[Vec<i64>],
) -> Vec<Vec<i64>> {
	let num_tasks = tasks.len();
	let num_cooks = cooks.len();
	let mut eff_duration = vec![vec![0i64; num_tasks]; num_cooks + 1];
	debug_assert!(required_skill.len() == num_tasks);

	for c in 0..=num_cooks {
		for (t, task) in tasks.iter().enumerate() {
			if c == 0 {
				eff_duration[c][t] = task.duration_minutes as i64;
			} else if let Some(ref map) = task.duration_by_skill {
				let si = required_skill[t];
				if si < 0 {
					eff_duration[c][t] = task.duration_minutes as i64;
				} else {
					let si_u = (si - 1) as usize;
					debug_assert!(si_u < cook_skill_level[c].len(),
						"cook {} skill index {} out of range 0..{}", c, si_u, cook_skill_level[c].len());
					let level = SkillLevel::iter()
						.nth(cook_skill_level[c][si_u] as usize)
						.expect("valid skill level index");
					eff_duration[c][t] =
						duration_for_skill(map, level).unwrap_or(task.duration_minutes) as i64;
				}
			} else {
				eff_duration[c][t] = task.duration_minutes as i64;
			}
		}
	}

	debug_assert!(eff_duration.len() == num_cooks + 1);
	debug_assert!(eff_duration.iter().all(|row| row.len() == num_tasks));
	debug_assert!(eff_duration.iter().all(|row| row.iter().all(|&v| v > 0)));

	eff_duration
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn build_dzn(
	num_tasks: usize,
	horizon: u32,
	num_cooks: usize,
	num_recipes: usize,
	num_deps: usize,
	num_equipment: usize,
	num_kinds: usize,
	max_resources: usize,
	num_skills: usize,
	num_preheats: usize,
	durations: &[i64],
	needs_cook_arr: &[bool],
	recipe_of: &[i64],
	deps_from: &[i64],
	deps_to: &[i64],
	equip_kind: &[i64],
	task_kinds_flat: &[i64],
	kind_start: &[i64],
	kind_end: &[i64],
	eff_duration: &[Vec<i64>],
	cook_skill_level: &[Vec<i64>],
	required_skill: &[i64],
	min_level: &[i64],
	preheat_tasks: &[i64],
	preheat_bakes: &[i64],
) -> String {
	let mut w = DznWriter::new();

	w.param("num_tasks", num_tasks);
	w.param("horizon", horizon);
	w.param("num_cooks", num_cooks);
	w.param("num_recipes", num_recipes);
	w.param("num_deps", num_deps);
	w.param("num_equipment", num_equipment);
	w.param("num_kinds", num_kinds);
	w.param("max_resources", max_resources.max(1));
	w.param("num_skills", num_skills);
	w.param("num_preheats", num_preheats);

	w.int_array("duration", 1, num_tasks, durations);
	w.bool_array("needs_cook", 1, num_tasks, needs_cook_arr);
	w.int_array("recipe_of", 1, num_tasks, recipe_of);
	w.int_array("deps_from", 1, num_deps, deps_from);
	w.int_array("deps_to", 1, num_deps, deps_to);
	w.int_array("equip_kind", 1, num_equipment, equip_kind);

	if max_resources > 0 {
		w.int_array2d("task_kinds", 1, num_tasks, 1, max_resources, task_kinds_flat);
	} else {
		// MiniZinc requires a valid 2D array even if empty
		w.int_array2d("task_kinds", 1, num_tasks, 1, 1, &[0i64]);
	}

	if num_kinds > 0 {
		w.int_array("kind_start", 1, num_kinds, kind_start);
		w.int_array("kind_end", 1, num_kinds, kind_end);
	} else {
		w.int_array("kind_start", 1, 1, &[0i64]);
		w.int_array("kind_end", 1, 1, &[0i64]);
	}

	let eff_flat: Vec<i64> = eff_duration.iter().flat_map(|row| row.iter().copied()).collect();
	w.int_array2d("eff_duration", 0, num_cooks, 1, num_tasks, &eff_flat);

	let csl_flat: Vec<i64> = cook_skill_level.iter().flat_map(|row| row.iter().copied()).collect();
	w.int_array2d("cook_skill_level", 0, num_cooks, 1, num_skills.max(1), &csl_flat);

	w.int_array("required_skill", 1, num_tasks, required_skill);
	w.int_array("min_level", 1, num_tasks, min_level);
	w.int_array("preheat_tasks", 1, num_preheats.max(1), preheat_tasks);
	w.int_array("preheat_bakes", 1, num_preheats.max(1), preheat_bakes);

	w.content
}

fn run_solver(model_input: &str, _tasks: &[TaskData]) -> Result<String, ScheduleError> {
	let mut child = Command::new("minizinc")
		.arg("--solver")
		.arg("gecode")
		.arg("--json-stream")
		.arg("--time-limit")
		.arg("10000")
		.arg("-")
		.stdin(Stdio::piped())
		.stdout(Stdio::piped())
		.stderr(Stdio::piped())
		.spawn()?;

	child
		.stdin
		.take()
		.expect("stdin configured")
		.write_all(model_input.as_bytes())?;

	let output = child.wait_with_output()?;
	let stderr = String::from_utf8_lossy(&output.stderr);
	let stdout = String::from_utf8_lossy(&output.stdout);

	if !output.status.success() {
		let exit_code = output.status.code();
		let stderr_lower = stderr.to_lowercase();

		if stderr_lower.contains("unsatisfiable") {
			return Err(ScheduleError::Unfeasible(format!(
				"Problem is unsatisfiable. stderr: {}",
				stderr
			)));
		}

		return Err(ScheduleError::SolverFailure(format!(
			"minizinc exited with code {:?}. stderr: {}\nstdout: {}\n(model omitted, {} chars)",
			exit_code,
			stderr,
			stdout,
			model_input.len()
		)));
	}

	let stderr_lower = stderr.to_lowercase();
	if stderr_lower.contains("unsatisfiable") {
		return Err(ScheduleError::Unfeasible(format!(
			"Problem is unsatisfiable. stderr: {}",
			stderr
		)));
	}

	if stdout.is_empty() {
		return Err(ScheduleError::NoSolution);
	}

	Ok(stdout.to_string())
}

#[allow(clippy::too_many_arguments)]
fn parse_solution(
	stdout: &str,
	tasks: &[TaskData],
	id_to_idx: &HashMap<String, usize>,
	cooks: &[Cook],
	equipment: &[EquipInfo],
	max_resources: usize,
	durations: &[i64],
	eff_duration: &[Vec<i64>],
	needs_cook_arr: &[bool],
) -> Result<Plan, ScheduleError> {
	let num_tasks = tasks.len();
	let assign_len = num_tasks * max_resources.max(1);
	let mut last_solution: Option<(Vec<u32>, Vec<usize>, Vec<usize>)> = None;

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
			if let Some(arr_str) = line.strip_prefix("start = ").and_then(|s| s.strip_suffix(';'))
				&& let Some(v) = parse_i64_array(arr_str)
			{
				start_vals = Some(v.into_iter().map(|x| x as u32).collect());
			}
			if let Some(arr_str) = line.strip_prefix("cook = ").and_then(|s| s.strip_suffix(';'))
				&& let Some(v) = parse_i64_array(arr_str)
			{
				cook_vals = Some(v.into_iter().map(|x| x as usize).collect());
			}
			if let Some(arr_str) = line.strip_prefix("assign = ").and_then(|s| s.strip_suffix(';'))
				&& let Some(v) = parse_i64_array(arr_str)
			{
				assign_vals = Some(v.into_iter().map(|x| x as usize).collect());
			}
		}

		if let (Some(sv), Some(cv), Some(av)) = (&start_vals, &cook_vals, &assign_vals)
			&& sv.len() == num_tasks && cv.len() == num_tasks && av.len() == assign_len
		{
			last_solution = Some((sv.clone(), cv.clone(), av.clone()));
		}
	}

	let (start_vals, cook_vals, assign_vals) = last_solution.ok_or(ScheduleError::NoSolution)?;

	let plan_tasks: Vec<Task> = tasks
		.iter()
		.enumerate()
		.map(|(i, task)| {
			let cook_name = {
				let ci = cook_vals[i];
				if ci > 0 && ci <= cooks.len() {
					Some(cooks[ci - 1].name.clone())
				} else {
					None
				}
			};

			let resource_ids: Vec<Option<String>> = (0..task.resource_kinds.len())
				.map(|r| {
					let ai = assign_vals[i * max_resources.max(1) + r];
					if ai > 0 && ai <= equipment.len() {
						Some(equipment[ai - 1].name.clone())
					} else {
						None
					}
				})
				.collect();

			let actual_dur = if needs_cook_arr[i] {
				eff_duration[cook_vals[i]][i] as u32
			} else {
				durations[i] as u32
			};

			let deps_ids: Vec<String> = task
				.dependencies
				.iter()
				.filter(|d| id_to_idx.contains_key(d.as_str()))
				.cloned()
				.collect();

			Task {
				id: task.id.clone(),
				dish: task.id.split(':').next().unwrap_or("").to_string(),
				description: task.description.clone(),
				start_offset_minutes: start_vals[i],
				duration_minutes: actual_dur,
				resource_ids,
				resource_kinds: task.resource_kinds.clone(),
				cook: cook_name,
				dependencies: deps_ids,
			}
		})
		.collect();

	Ok(Plan { tasks: plan_tasks })
}

fn parse_i64_array(s: &str) -> Option<Vec<i64>> {
	let s = s.trim();
	if !s.starts_with('[') || !s.ends_with(']') {
		return None;
	}
	let inner = &s[1..s.len() - 1];
	if inner.is_empty() {
		return Some(Vec::new());
	}
	inner.split(',').map(|n| n.trim().parse::<i64>().ok()).collect()
}

fn expand_tasks(recipes: &[Recipe], kitchen: &Kitchen) -> (Vec<TaskData>, Vec<PreHeatPair>) {
	let mut tasks = Vec::new();

	for (ri, recipe) in recipes.iter().enumerate() {
		for step in &recipe.steps {
			let tid = format!("{}:{}", recipe.name, step.id);
			let deps: Vec<String> = step
				.dependencies
				.iter()
				.map(|d| format!("{}:{}", recipe.name, d))
				.collect();
			tasks.push(TaskData {
				id: tid,
				description: step.description.clone(),
				duration_minutes: step.duration_minutes,
				resource_kinds: step.resource_kinds.clone(),
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

	let preheat_pairs = inject_preheat_tasks(&mut tasks, kitchen);

	(tasks, preheat_pairs)
}

struct PreHeatPair {
	preheat_idx: usize,
	bake_idx: usize,
}

fn inject_preheat_tasks(tasks: &mut Vec<TaskData>, kitchen: &Kitchen) -> Vec<PreHeatPair> {
	let mut kind_temps: HashMap<String, Vec<u16>> = HashMap::new();
	let mut temp_to_bakes: HashMap<(String, u16), Vec<usize>> = HashMap::new();

	for (i, task) in tasks.iter().enumerate() {
		if let Some(temp) = task.temperature_celsius
			&& let Some(kind) = task.resource_kinds.first()
		{
			kind_temps.entry(kind.clone()).or_default().push(temp);
			temp_to_bakes.entry((kind.clone(), temp)).or_default().push(i);
		}
	}

	if kind_temps.is_empty() {
		return Vec::new();
	}

	let mut preheat_pairs = Vec::new();

	for (kind, temps) in &kind_temps {
		let min_rate = kitchen
			.equipment
			.iter()
			.filter(|e| e.kind == *kind)
			.map(|e| e.preheat_rate_minutes_per_celsius)
			.fold(f64::INFINITY, f64::min);

		if min_rate <= 0.0 || !min_rate.is_finite() {
			continue;
		}

		let mut unique_temps: Vec<u16> = temps.clone();
		unique_temps.sort();
		unique_temps.dedup();

		let mut prev_temp = kitchen.ambient_temperature_celsius as u16;
		let mut prev_preheat_id: Option<String> = None;

		for &temp in &unique_temps {
			let delta = (temp as f64 - prev_temp as f64).max(0.0);
			let duration = (min_rate * delta).round() as u32;
			let preheat_id = format!("{}:preheat:{}", kind, temp);

			let mut deps = Vec::new();
			if let Some(ref prev_id) = prev_preheat_id {
				deps.push(prev_id.clone());
			}

			let bake_idx = temp_to_bakes[&(kind.clone(), temp)][0];
			debug_assert!(bake_idx < tasks.len(), "preheat bake index {} out of range 0..{}", bake_idx, tasks.len());
			let preheat_idx = tasks.len();

			tasks.push(TaskData {
				id: preheat_id.clone(),
				description: format!("Pre-heat {} to {}°C", kind, temp),
				duration_minutes: duration,
				resource_kinds: vec![kind.clone()],
				dependencies: deps,
				recipe_idx: tasks[bake_idx].recipe_idx,
				needs_cook: false,
				duration_by_skill: None,
				skill: None,
				min_skill_level: None,
				temperature_celsius: None,
			});

			for &bi in &temp_to_bakes[&(kind.clone(), temp)] {
				debug_assert!(bi < tasks.len(), "preheat bake dep target {} out of range 0..{}", bi, tasks.len());
				tasks[bi].dependencies.push(preheat_id.clone());
				preheat_pairs.push(PreHeatPair { preheat_idx, bake_idx: bi });
			}

			prev_temp = temp;
			prev_preheat_id = Some(preheat_id);
		}
	}

	debug_assert!(preheat_pairs.iter().all(|p| p.preheat_idx < tasks.len() && p.bake_idx < tasks.len()));

	preheat_pairs
}

#[cfg(test)]
mod tests {
	use super::*;

	fn dummy_task(id: &str, deps: Vec<&str>) -> TaskData {
		TaskData {
			id: id.to_string(),
			description: String::new(),
			duration_minutes: 10,
			resource_kinds: vec![],
			dependencies: deps.into_iter().map(|d| d.to_string()).collect(),
			recipe_idx: 0,
			needs_cook: true,
			duration_by_skill: None,
			skill: None,
			min_skill_level: None,
			temperature_celsius: None,
		}
	}

	fn dummy_equip(kind: &str) -> EquipInfo {
		EquipInfo { name: kind.to_string(), kind: kind.to_string() }
	}

	#[test]
	fn build_dependencies_empty() {
		let tasks = vec![dummy_task("a", vec![])];
		let id_to_idx: HashMap<_, _> = [("a".to_string(), 0)].into();
		let (from, to) = build_dependencies(&tasks, &id_to_idx);
		assert!(from.is_empty());
		assert!(to.is_empty());
	}

	#[test]
	fn build_dependencies_linear_chain() {
		let tasks = vec![
			dummy_task("a", vec![]),
			dummy_task("b", vec!["a"]),
			dummy_task("c", vec!["b"]),
		];
		let id_to_idx: HashMap<_, _> =
			[("a".to_string(), 0), ("b".to_string(), 1), ("c".to_string(), 2)].into();
		let (from, to) = build_dependencies(&tasks, &id_to_idx);
		assert_eq!(from, vec![1i64, 2i64]);
		assert_eq!(to, vec![2i64, 3i64]);
	}

	#[test]
	fn build_dependencies_fork() {
		let tasks = vec![
			dummy_task("a", vec![]),
			dummy_task("b", vec!["a"]),
			dummy_task("c", vec!["a"]),
		];
		let id_to_idx: HashMap<_, _> =
			[("a".to_string(), 0), ("b".to_string(), 1), ("c".to_string(), 2)].into();
		let (from, to) = build_dependencies(&tasks, &id_to_idx);
		assert_eq!(from, vec![1i64, 1i64]);
		assert_eq!(to, vec![2i64, 3i64]);
	}

	#[test]
	fn build_dependencies_ignores_missing_deps() {
		let tasks = vec![dummy_task("a", vec!["missing"])];
		let id_to_idx: HashMap<_, _> = [("a".to_string(), 0)].into();
		let (from, to) = build_dependencies(&tasks, &id_to_idx);
		assert!(from.is_empty());
		assert!(to.is_empty());
	}

	#[test]
	fn build_dependencies_deduplicates() {
		let tasks = vec![
			dummy_task("a", vec![]),
			dummy_task("b", vec!["a"]),
			dummy_task("c", vec!["b", "a"]),
		];
		let id_to_idx: HashMap<_, _> =
			[("a".to_string(), 0), ("b".to_string(), 1), ("c".to_string(), 2)].into();
		let (from, to) = build_dependencies(&tasks, &id_to_idx);
		assert_eq!(from, vec![1i64, 2i64, 1i64]);
		assert_eq!(to, vec![2i64, 3i64, 3i64]);
	}

	#[test]
	fn build_equip_kind_mapping_single_kind() {
		let equip = vec![dummy_equip("oven")];
		let (kind_to_idx, kinds) = build_equip_kind_mapping(&equip);
		assert_eq!(kind_to_idx.len(), 1);
		assert_eq!(*kind_to_idx.get("oven").unwrap(), 1);
		assert_eq!(kinds, vec![1i64]);
	}

	#[test]
	fn build_equip_kind_mapping_multiple_kinds() {
		let equip = vec![
			dummy_equip("burner"),
			dummy_equip("burner"),
			dummy_equip("oven"),
			dummy_equip("pot"),
		];
		let (kind_to_idx, kinds) = build_equip_kind_mapping(&equip);
		assert_eq!(kind_to_idx.len(), 3);
		assert_eq!(kinds, vec![1i64, 1i64, 2i64, 3i64]);
	}

	#[test]
	fn build_kind_ranges_single_per_kind() {
		let (start, end) = build_kind_ranges(&[1i64, 2i64, 3i64], 3);
		assert_eq!(start, vec![1i64, 2i64, 3i64]);
		assert_eq!(end, vec![1i64, 2i64, 3i64]);
	}

	#[test]
	fn build_kind_ranges_multiple_per_kind() {
		let (start, end) = build_kind_ranges(&[1i64, 1i64, 2i64, 2i64, 2i64], 2);
		assert_eq!(start, vec![1i64, 3i64]);
		assert_eq!(end, vec![2i64, 5i64]);
	}

	#[test]
	fn build_task_kinds_known_and_unknown() {
		let mut kind_to_idx: HashMap<String, usize> = HashMap::new();
		kind_to_idx.insert("oven".to_string(), 1);
		kind_to_idx.insert("burner".to_string(), 2);
		let tasks = vec![
			TaskData { resource_kinds: vec!["oven".into()], ..dummy_task("a", vec![]) },
			TaskData {
				resource_kinds: vec!["burner".into(), "pot".into()],
				..dummy_task("b", vec![])
			},
			TaskData { resource_kinds: vec!["unknown".into()], ..dummy_task("c", vec![]) },
		];
		let result = build_task_kinds(&tasks, &kind_to_idx);
		assert_eq!(result[0], vec![1usize]);
		assert_eq!(result[1], vec![2usize, 0usize]);
		assert_eq!(result[2], vec![0usize]);
	}

	#[test]
	fn build_skill_data_no_skills() {
		let tasks = vec![dummy_task("a", vec![])];
		let cooks = vec![];
		let (_, num, required, min_level, csl) = build_skill_data(&tasks, &cooks);
		assert_eq!(num, 0);
		assert_eq!(required, vec![-1i64]);
		assert_eq!(min_level, vec![0i64]);
		assert_eq!(csl, vec![vec![0i64]]);
	}

	#[test]
	fn build_skill_data_single_skill() {
		let tasks = vec![TaskData {
			skill: Some("knife_work".into()),
			min_skill_level: Some(SkillLevel::Novice),
			..dummy_task("a", vec![])
		}];
		let cooks = vec![Cook {
			name: "Alice".into(),
			skills: [("knife_work".into(), SkillLevel::Advanced)].into(),
		}];
		let (_, num, required, min_level, _csl) = build_skill_data(&tasks, &cooks);
		assert_eq!(num, 1);
		assert_eq!(required, vec![1i64]);
		assert_eq!(min_level, vec![SkillLevel::Novice as u8 as i64]);
	}

	#[test]
	fn compute_effective_durations_no_skill() {
		let tasks = vec![dummy_task("a", vec![])];
		let cooks = vec![Cook { name: "Bob".into(), skills: [].into() }];
		let required_skill = vec![-1i64];
		let csl = vec![vec![0i64], vec![0i64]];
		let result = compute_effective_durations(&tasks, &cooks, &required_skill, &csl);
		assert_eq!(result.len(), 2);
		assert_eq!(result[0], vec![10i64]);
		assert_eq!(result[1], vec![10i64]);
	}

	#[test]
	fn compute_effective_durations_skill_fallback() {
		use std::collections::HashMap;
		let tasks = vec![TaskData {
			duration_minutes: 20,
			duration_by_skill: Some(HashMap::from([
				(SkillLevel::Unskilled, 20),
				(SkillLevel::Intermediate, 10),
			])),
			skill: Some("knife_work".into()),
			..dummy_task("a", vec![])
		}];
		let cooks = vec![
			Cook {
				name: "NoviceCook".into(),
				skills: [("knife_work".into(), SkillLevel::Novice)].into(),
			},
			Cook {
				name: "ExpertCook".into(),
				skills: [("knife_work".into(), SkillLevel::Expert)].into(),
			},
		];
		let required_skill = vec![1i64];
		let csl = vec![vec![0i64], vec![1i64], vec![4i64]];
		let result = compute_effective_durations(&tasks, &cooks, &required_skill, &csl);
		assert_eq!(result[0], vec![20i64]); // base
		assert_eq!(result[1], vec![20i64]); // Novice → Unskilled fallback
		assert_eq!(result[2], vec![10i64]); // Expert → Intermediate fallback
	}

	#[test]
	fn inject_preheat_tasks_no_temperature() {
		let mut tasks = vec![dummy_task("a", vec![])];
		let kitchen = Kitchen {
			equipment: vec![],
			ambient_temperature_celsius: 20.0,
			food: vec![],
			materials: vec![],
		};
		let pairs = inject_preheat_tasks(&mut tasks, &kitchen);
		assert!(pairs.is_empty());
		assert_eq!(tasks.len(), 1);
	}

	#[test]
	fn inject_preheat_tasks_single_temperature() {
		let mut tasks = vec![TaskData {
			temperature_celsius: Some(180),
			resource_kinds: vec!["oven".into()],
			..dummy_task("bake", vec![])
		}];
		let kitchen = Kitchen {
			equipment: vec![crate::models::kitchen::Equipment {
				id: "ov-1".into(),
				name: "Main Oven".into(),
				kind: "oven".into(),
				preheat_rate_minutes_per_celsius: 0.1,
			}],
			ambient_temperature_celsius: 20.0,
			food: vec![],
			materials: vec![],
		};
		let pairs = inject_preheat_tasks(&mut tasks, &kitchen);
		assert_eq!(tasks.len(), 2);
		assert_eq!(pairs.len(), 1);
		assert_eq!(pairs[0].preheat_idx, 1);
		assert_eq!(pairs[0].bake_idx, 0);
		assert_eq!(tasks[1].duration_minutes, 16); // (180 - 20) * 0.1
		assert!(tasks[1].description.starts_with("Pre-heat"));
		assert!(tasks[0].dependencies.contains(&tasks[1].id));
	}

	#[test]
	fn inject_preheat_tasks_chained_temperatures() {
		let mut tasks = vec![
			TaskData {
				temperature_celsius: Some(180),
				resource_kinds: vec!["oven".into()],
				..dummy_task("bake1", vec![])
			},
			TaskData {
				temperature_celsius: Some(200),
				resource_kinds: vec!["oven".into()],
				..dummy_task("bake2", vec![])
			},
		];
		let kitchen = Kitchen {
			equipment: vec![crate::models::kitchen::Equipment {
				id: "ov-1".into(),
				name: "Main Oven".into(),
				kind: "oven".into(),
				preheat_rate_minutes_per_celsius: 0.1,
			}],
			ambient_temperature_celsius: 20.0,
			food: vec![],
			materials: vec![],
		};
		let pairs = inject_preheat_tasks(&mut tasks, &kitchen);
		assert_eq!(tasks.len(), 4);
		assert_eq!(pairs.len(), 2);
		// First pre-heat (idx=2): (180 - 20) * 0.1 = 16, no deps
		assert_eq!(tasks[2].duration_minutes, 16);
		assert!(tasks[2].dependencies.is_empty());
		// Second pre-heat (idx=3): (200 - 180) * 0.1 = 2, depends on first
		assert_eq!(tasks[3].duration_minutes, 2);
		assert!(tasks[3].dependencies.contains(&tasks[2].id));
		// Bake tasks depend on their respective pre-heats
		assert!(tasks[0].dependencies.contains(&tasks[2].id));
		assert!(tasks[1].dependencies.contains(&tasks[3].id));
	}
}
