use std::collections::HashSet;

use crate::models::cook::Cook;
use crate::models::kitchen::Kitchen;
use crate::models::recipe::Recipe;

#[derive(Debug, serde::Serialize)]
pub struct ValidationError {
	pub error_type: String,
	pub message: String,
}

pub fn validate(kitchen: &Kitchen, cooks: &[Cook], recipes: &[Recipe]) -> Vec<ValidationError> {
	let mut errors = Vec::new();

	if recipes.is_empty() {
		errors.push(ValidationError {
			error_type: "no_recipes".into(),
			message: "At least one recipe is required".into(),
		});
		return errors;
	}

	if kitchen.equipment.is_empty() {
		errors.push(ValidationError {
			error_type: "no_equipment".into(),
			message: "Kitchen has no equipment selected".into(),
		});
	}

	let equip_kinds: HashSet<&str> = kitchen.equipment.iter().map(|e| e.kind.as_str()).collect();

	for recipe in recipes {
		for step in &recipe.steps {
			let task_id = format!("{}:{}", recipe.name, step.id);

			for kind in &step.resource_kinds {
				if !equip_kinds.contains(kind.as_str()) {
					errors.push(ValidationError {
						error_type: "missing_equipment_kind".into(),
						message: format!("No {} available for '{}'", kind, task_id),
					});
				}
			}

			if step.needs_cook {
				if cooks.is_empty() {
					errors.push(ValidationError {
						error_type: "no_cooks_for_task".into(),
						message: format!("No cooks provided but '{}' requires one", task_id),
					});
					continue;
				}

				if let Some(ref skill) = step.skill
					&& let Some(min_level) = step.min_skill_level
				{
					let qualified = cooks
						.iter()
						.any(|c| c.skills.get(skill).is_some_and(|level| *level >= min_level));
					if !qualified {
						let names: Vec<&str> = cooks.iter().map(|c| c.name.as_str()).collect();
						errors.push(ValidationError {
							error_type: "cook_skill_insufficient".into(),
							message: format!(
								"No cook meets the '{}' ≥ {:?} requirement for '{}' (available cooks: {})",
								skill,
								min_level,
								task_id,
								names.join(", "),
							),
						});
					}
				}
			}
		}
	}

	errors
}
