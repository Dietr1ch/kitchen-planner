use crate::plan::Plan;

pub trait Renderer {
    fn render(&self, plan: &Plan) -> String;
}

pub(crate) fn sorted_tasks(plan: &Plan) -> (Vec<crate::plan::Task>, u32) {
    let mut tasks = plan.tasks.clone();
    tasks.sort_by_key(|t| t.start_offset_minutes);

    let total_duration = tasks
        .iter()
        .map(|t| t.start_offset_minutes + t.duration_minutes)
        .max()
        .unwrap_or(0)
        .max(1);

    (tasks, total_duration)
}

pub(crate) fn truncate(s: &str, max_bytes: usize) -> String {
    if s.len() <= max_bytes {
        return s.to_string();
    }
    let end = s
        .char_indices()
        .take_while(|(i, _)| *i < max_bytes)
        .last()
        .map(|(i, c)| i + c.len_utf8())
        .unwrap_or(0);
    format!("{}…", &s[..end])
}

pub(crate) fn short_deps(deps: &[String]) -> String {
    deps.iter()
        .map(|d| d.rsplit(':').next().unwrap_or(d))
        .collect::<Vec<_>>()
        .join(", ")
}

mod text;
pub use text::TextRenderer;

mod html;
pub use html::HtmlRenderer;
