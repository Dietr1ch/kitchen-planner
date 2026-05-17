use super::{Renderer, SortOrder, short_deps, sorted_tasks, truncate};
use crate::models::plan::Plan;

pub struct TextRenderer {
	pub sort_order: SortOrder,
}

impl TextRenderer {
	pub fn new(sort_order: SortOrder) -> Self {
		TextRenderer { sort_order }
	}
}

impl Renderer for TextRenderer {
	fn render(&self, plan: &Plan) -> String {
		let (tasks, total_duration) = sorted_tasks(plan, self.sort_order);

		let bar_width = 40usize;
		let mut out = String::new();

		out.push_str("Kitchen Planner - Gantt Chart\n\n");
		out.push_str(&format!(
			"{:<bar_width$} │ {:<12} │ {:<38} │ {:<14} │ {:<10} │ {}\n",
			"Duration",
			"Dish",
			"Task",
			"Deps",
			"Cook",
			"Resource",
			bar_width = bar_width
		));
		out.push_str(&format!(
			"{} ┼ {} ┼ {} ┼ {} ┼ {} ┼ {}\n",
			"─".repeat(bar_width),
			"─".repeat(12),
			"─".repeat(38),
			"─".repeat(14),
			"─".repeat(10),
			"─".repeat(10)
		));

		for task in &tasks {
			let start = task.start_offset_minutes;
			let end = start + task.duration_minutes;

			let bar_len = ((task.duration_minutes as f64 / total_duration as f64)
				* bar_width as f64)
				.round()
				.max(1.0) as usize;

			let offset = (start as f64 / total_duration as f64 * bar_width as f64).round() as usize;

			let time_str = format!("{:>3}–{:<3}", start, end);
			let offset = offset.min(bar_width.saturating_sub(time_str.len() + 2));
			let max_bar = bar_width.saturating_sub(offset + time_str.len() + 1);
			let bar_chars = "▓".repeat(bar_len.min(max_bar));
			let bar_empty = bar_chars.is_empty();
			let bar_full = format!(
				"{}{}{}{}",
				" ".repeat(offset),
				time_str,
				if bar_empty { "" } else { " " },
				bar_chars,
			);
			let bar_column = format!(
				"{:<width$}",
				bar_full.chars().take(bar_width).collect::<String>(),
				width = bar_width
			);

			let dish = truncate(&task.dish, 12);
			let desc = truncate(&task.description, 35);
			let deps = short_deps(&task.dependencies);
			let resource = resource_display(&task.resource_kinds, &task.resource_ids);
			let cook = task.cook.as_deref().unwrap_or("(none)");

			out.push_str(&format!(
				"{} │ {:<12} │ {:<38} │ {:<14} │ {:<10} │ {}\n",
				bar_column, dish, desc, deps, cook, resource
			));
		}

		out
	}
}

fn resource_display(kinds: &[String], names: &[Option<String>]) -> String {
	if kinds.is_empty() {
		return "(none)".to_string();
	}
	kinds
		.iter()
		.zip(names.iter())
		.map(|(k, n)| match n {
			Some(name) => format!("{} ({})", k, name),
			None => k.clone(),
		})
		.collect::<Vec<_>>()
		.join(", ")
}
