use std::collections::HashMap;

use crate::cook::Cook;
use crate::kitchen::Kitchen;
use crate::plan::{Plan, Task};
use crate::recipe::Recipe;

pub fn schedule(kitchen: &Kitchen, cooks: &[Cook], recipes: &[Recipe]) -> Plan {
    let mut tasks = Vec::new();

    let mut resource_available: HashMap<String, u32> = kitchen
        .equipment
        .iter()
        .map(|e| (e.id.clone(), 0))
        .collect();

    let mut cook_available: HashMap<String, u32> = cooks
        .iter()
        .map(|c| (c.name.clone(), 0))
        .collect();

    for recipe in recipes {
        schedule_recipe(
            recipe,
            &mut resource_available,
            &mut cook_available,
            &mut tasks,
        );
    }

    Plan {
        start_time: "18:00".to_string(),
        tasks,
    }
}

fn pick_cook(cook_available: &HashMap<String, u32>) -> Option<String> {
    cook_available
        .iter()
        .min_by_key(|&(_, time)| *time)
        .map(|(name, _)| name.clone())
}

fn schedule_recipe(
    recipe: &Recipe,
    resource_available: &mut HashMap<String, u32>,
    cook_available: &mut HashMap<String, u32>,
    tasks: &mut Vec<Task>,
) {
    let mut step_finish: HashMap<String, u32> = HashMap::new();

    for step in topological_sort(&recipe.steps) {
        let dep_finish = step
            .dependencies
            .iter()
            .filter_map(|d| step_finish.get(d))
            .max()
            .copied()
            .unwrap_or(0);

        let resource_ready = step
            .resource_id
            .as_ref()
            .and_then(|rid| resource_available.get(rid).copied())
            .unwrap_or(0);

        let cook_name = pick_cook(cook_available);
        let cook_ready = cook_name
            .as_ref()
            .and_then(|name| cook_available.get(name))
            .copied()
            .unwrap_or(0);

        let start = dep_finish.max(resource_ready).max(cook_ready);
        let finish = start + step.duration_minutes;

        if let Some(rid) = &step.resource_id {
            resource_available.insert(rid.clone(), finish);
        }
        if let Some(ref name) = cook_name {
            cook_available.insert(name.clone(), finish);
        }

        step_finish.insert(step.id.clone(), finish);

        let task_prefix = &recipe.name;

        tasks.push(Task {
            id: format!("{}:{}", task_prefix, step.id),
            description: format!("{}: {}", task_prefix, step.description),
            start_offset_minutes: start,
            duration_minutes: step.duration_minutes,
            resource_id: step.resource_id.clone(),
            cook: cook_name,
            dependencies: step
                .dependencies
                .iter()
                .map(|dep_id| format!("{}:{}", task_prefix, dep_id))
                .collect(),
        });
    }
}

fn topological_sort(steps: &[crate::recipe::Step]) -> Vec<crate::recipe::Step> {
    let step_map: HashMap<&str, &crate::recipe::Step> =
        steps.iter().map(|s| (s.id.as_str(), s)).collect();

    let mut in_degree: HashMap<&str, usize> = HashMap::new();
    let mut dependents: HashMap<&str, Vec<&str>> = HashMap::new();

    for step in steps {
        in_degree.entry(step.id.as_str()).or_insert(0);
        for dep in &step.dependencies {
            dependents.entry(dep).or_default().push(step.id.as_str());
            *in_degree.entry(step.id.as_str()).or_insert(0) += 1;
        }
    }

    let mut queue: Vec<&str> = in_degree
        .iter()
        .filter(|entry| *entry.1 == 0)
        .map(|entry| *entry.0)
        .collect();

    let mut sorted = Vec::new();

    while let Some(id) = queue.pop() {
        if let Some(step) = step_map.get(id) {
            sorted.push((*step).clone());
        }
        if let Some(deps) = dependents.get(id) {
            for &dep_id in deps {
                if let Some(deg) = in_degree.get_mut(dep_id) {
                    *deg = deg.saturating_sub(1);
                    if *deg == 0 {
                        queue.push(dep_id);
                    }
                }
            }
        }
    }

    sorted
}
