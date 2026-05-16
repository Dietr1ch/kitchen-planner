use super::{Renderer, SortOrder, short_deps, sorted_tasks};
use crate::models::plan::Plan;

pub struct HtmlRenderer;

impl Renderer for HtmlRenderer {
	fn render(&self, plan: &Plan) -> String {
		let (tasks, total_duration) = sorted_tasks(plan, SortOrder::Start);

		let total_f = total_duration as f64;
		let mut rows = String::new();
		for task in &tasks {
			let start = task.start_offset_minutes;
			let end = start + task.duration_minutes;

			let offset_pct = (start as f64 / total_f * 100.0).round();
			let width_pct = ((task.duration_minutes as f64 / total_f) * 100.0)
				.round()
				.max(1.0);

			let resource =
				resource_display(task.resource_kind.as_deref(), task.resource_id.as_deref());
			let cook = task.cook.as_deref().unwrap_or("(none)");
			let dish = html_escape(&task.dish);
			let desc = html_escape(&task.description);
			let deps = short_deps(&task.dependencies);

			let task_id = html_escape(&task.id);
			let dep_ids = task
				.dependencies
				.iter()
				.map(|d| html_escape(d))
				.collect::<Vec<_>>()
				.join(", ");

			let cook_cell = if cook == "(none)" {
				cook.to_string()
			} else {
				format!(
					"<span class=\"cook-badge\" style=\"background: {};\">{}</span>",
					cook_color(cook),
					cook,
				)
			};

			let bar_label = format!("{}–{}", start, end);

			rows.push_str(&format!(
				concat!(
					"<tr data-task-id=\"{}\" data-depends-on=\"{}\" data-start=\"{}\" data-cook=\"{}\">",
					"<td><div class=\"bar-container\">",
					"<div class=\"bar\" style=\"margin-left: {:.0}%; width: {:.0}%;\">{}</div>",
					"</div></td>",
					"<td>{}</td>",
					"<td>{}</td>",
					"<td>{}</td>",
					"<td>{}</td>",
					"<td>{}</td>",
					"</tr>\n",
				),
				task_id,
				dep_ids,
				start,
				html_escape(cook),
				offset_pct,
				width_pct,
				bar_label,
				dish,
				desc,
				deps,
				cook_cell,
				resource,
			));
		}

		format!(
			concat!(
				"<!DOCTYPE html>\n",
				"<html lang=\"en\">\n",
				"<head>\n",
				"<meta charset=\"UTF-8\">\n",
				"<title>Kitchen Planner - Gantt Chart</title>\n",
				"<style>\n",
				"body {{ font-family: sans-serif; margin: 2rem; }}\n",
				"h1 {{ color: #333; }}\n",
				"p {{ color: #666; }}\n",
				"table {{ border-collapse: collapse; width: 100%; }}\n",
				"th {{ background: #f5f5f5; text-align: left; padding: 8px; border-bottom: 2px solid #ddd; }}\n",
				"th[data-sort] {{ cursor: pointer; }}\n",
				"th[data-sort]:hover {{ background: #e8e8e8; }}\n",
				"td {{ padding: 8px; border-bottom: 1px solid #eee; vertical-align: middle; }}\n",
				"tr:hover {{ background: #fafafa; }}\n",
				"tr.dep-upstream {{ background: #fff3e0 !important; }}\n",
				"tr.dep-upstream .bar {{ background: #ff9800 !important; }}\n",
				"tr.dep-downstream {{ background: #e3f2fd !important; }}\n",
				"tr.dep-downstream .bar {{ background: #2196f3 !important; }}\n",
				".bar-container {{ background: #f0f0f0; border-radius: 4px; height: 24px; position: relative; min-width: 200px; overflow: hidden; }}\n",
				".bar {{ background: #4caf50; height: 24px; border-radius: 4px; display: flex; align-items: center; padding: 0 8px; color: white; font-size: 12px; white-space: nowrap; box-sizing: border-box; min-width: fit-content; cursor: pointer; }}\n",
				".cook-badge {{ display: inline-block; padding: 2px 8px; border-radius: 4px; color: #fff; font-weight: 500; font-size: 0.85rem; }}\n",
				"</style>\n",
				"</head>\n",
				"<body>\n",
				"<h1>Kitchen Planner - Gantt Chart</h1>\n",
				"<table id=\"gantt\">\n",
				"<thead><tr><th data-sort=\"start\">Duration \u{2195}</th><th>Dish</th><th>Task</th><th>Dependencies</th><th data-sort=\"cook\">Cook \u{2195}</th><th>Resource</th></tr></thead>\n",
				"<tbody>\n",
				"{}</tbody>\n",
				"</table>\n",
				"<script>\n",
				"(function() {{\n",
				"  var table = document.getElementById('gantt');\n",
				"  var tbody = table.querySelector('tbody');\n",
				"  table.querySelectorAll('th[data-sort]').forEach(function(th) {{\n",
				"    th.addEventListener('click', function() {{\n",
				"      var key = this.dataset.sort;\n",
				"      var rows = Array.from(tbody.querySelectorAll('tr'));\n",
				"      rows.sort(function(a, b) {{\n",
				"        var va = a.getAttribute('data-' + key) || '';\n",
				"        var vb = b.getAttribute('data-' + key) || '';\n",
				"        if (key === 'start') return Number(va) - Number(vb);\n",
				"        return va.localeCompare(vb);\n",
				"      }});\n",
				"      rows.forEach(function(row) {{ tbody.appendChild(row); }});\n",
				"    }});\n",
				"  }});\n",
				"\n",
				"  table.addEventListener('mouseover', function(e) {{\n",
				"    var row = e.target.closest('tr[data-task-id]');\n",
				"    if (!row) return;\n",
				"    var taskId = row.dataset.taskId;\n",
				"    var deps = (row.dataset.dependsOn || '').split(/,\\s*/).filter(Boolean);\n",
				"    deps.forEach(function(id) {{\n",
				"        var dep = document.querySelector('tr[data-task-id=\"' + id + '\"]');\n",
				"        if (dep) dep.classList.add('dep-upstream');\n",
				"    }});\n",
				"    document.querySelectorAll('tr[data-depends-on]').forEach(function(other) {{\n",
				"        var otherDeps = (other.dataset.dependsOn || '').split(/,\\s*/).filter(Boolean);\n",
				"        if (otherDeps.indexOf(taskId) !== -1) {{\n",
				"            other.classList.add('dep-downstream');\n",
				"        }}\n",
				"    }});\n",
				"  }});\n",
				"  table.addEventListener('mouseout', function(e) {{\n",
				"    var row = e.target.closest('tr[data-task-id]');\n",
				"    if (!row) return;\n",
				"    document.querySelectorAll('.dep-upstream, .dep-downstream').forEach(function(el) {{\n",
				"        el.classList.remove('dep-upstream', 'dep-downstream');\n",
				"    }});\n",
				"  }});\n",
				"}})();\n",
				"</script>\n",
				"</body>\n",
				"</html>\n",
			),
			rows,
		)
	}
}

fn cook_color(name: &str) -> String {
	let hash: u32 = name
		.bytes()
		.fold(0u32, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u32));
	let hue = hash % 360;
	format!("hsl({}, 60%, 50%)", hue)
}

fn resource_display(kind: Option<&str>, name: Option<&str>) -> String {
	match (kind, name) {
		(Some(k), Some(n)) => format!("{} ({})", k, n),
		(Some(k), None) => k.to_string(),
		(None, Some(n)) => n.to_string(),
		(None, None) => "(none)".to_string(),
	}
}

fn html_escape(s: &str) -> String {
	s.replace('&', "&amp;")
		.replace('<', "&lt;")
		.replace('>', "&gt;")
		.replace('"', "&quot;")
}
