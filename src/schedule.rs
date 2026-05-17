use crate::models::cook::Cook;
use crate::models::kitchen::Kitchen;
use crate::models::plan::Plan;
use crate::models::recipe::Recipe;

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

/// Given a Kitchen, a group of Cooks and some desired Recipes finds a plan
/// to cook the recipes in such a way that,
/// - We align the ends, so that every dish becomes ready at the same time.
/// - We consider oven and grill preheating for steps with a target temperature.
///   - There's extra preheating or cooldown steps so baking and grilling is repeatable.
///   - And we avoid preheating until it's necessary.
pub fn schedule(
	kitchen: &Kitchen,
	cooks: &[Cook],
	recipes: &[Recipe],
) -> Result<Plan, ScheduleError> {
	// TODO: Build a model for the problem
	// TODO: Model the instance
	// TODO: Pass both into the minizinc+gecode and return a plan

	// NOTE: We can call minizinc and pipe the problem into it to avoid dealing
	//       with temporary files,
	//
	//     minizinc \
	//       --statistics \
	//       --compiler-statistics \
	//       --json-stream \
	//       --time-limit 10000 \
	//       -

	// Resources,
	// - https://docs.minizinc.dev/en/stable/part_2_tutorial.html
	Err(ScheduleError::NoSolution)
}
