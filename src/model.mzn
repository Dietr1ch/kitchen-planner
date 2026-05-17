% Kitchen Planner — Scheduling Model
% ==================================
%
% Constraint programming model for scheduling multiple recipes across a
% kitchen with limited equipment and cooks.  The solver decides task start
% times, cook assignments, and equipment assignments, then minimises a
% weighted combination of makespan, recipe-end alignment, and pre-heat
% waste.
%
% All parameters are supplied as DZN data from the Rust pre-processor
% (see :/src/schedule.rs). Arrays use MiniZinc's native 1-based indexing
% throughout.



% Parameters  (supplied as DZN data)
% ==========

% Dimensions
% ----------
int: num_tasks;       % Total number of tasks (including synthetic pre-heats)
int: horizon;         % Upper bound on start times (sum of all durations × 2)
int: num_cooks;       % Number of real cooks (index 0 = dummy "no cook")
int: num_recipes;     % Number of recipes to schedule
int: num_deps;        % Number of dependency edges
int: num_equipment;   % Number of equipment items in the kitchen
int: num_kinds;       % Number of distinct equipment kinds (e.g. oven, burner)
int: max_resources;   % Max number of resource-slots any task requires
int: num_skills;      % Number of distinct skill names across all tasks
int: num_preheats;    % Number of synthetic pre-heat tasks injected

% Task data
% ---------
% duration: Task->Duration map. Base duration of each task in minutes (used when no skill override applies)
array[1..num_tasks] of int: duration;
% needs_cook: Task->Bool map. Whether the task occupies a cook. False for oven/rest/thaw/cooling.
array[1..num_tasks] of bool: needs_cook;
% recipe_of: Task->Recipe map. index (1..num_recipes) that this task belongs to.
array[1..num_tasks] of int: recipe_of;

% Dependency edges
% ----------------
% deps_from: Parent task index for each dependency edge.
array[1..num_deps] of int: deps_from;
% deps_to: Child task index for each dependency edge.
array[1..num_deps] of int: deps_to;

% Equipment kind mapping
% ----------------------
% equip_kind: Kind index (1..num_kinds) of each equipment item.
array[1..num_equipment] of int: equip_kind;
% task_kinds: Equipment-kind slot for each (task, resource).  0 = unused slot.
array[1..num_tasks, 1..max_resources] of int: task_kinds;
% kind_start: First equipment index belonging to each kind.
array[1..num_kinds] of int: kind_start;
% kind_end: Last equipment index belonging to each kind.
array[1..num_kinds] of int: kind_end;

% Skill data
% ----------
array[0..num_cooks, 1..num_tasks] of int: eff_duration;
  % Pre-computed duration for each (cook, task) pair, including skill-adaptive
  % overrides.  Row 0 is the no-cook fallback (plain base duration).
array[0..num_cooks, 1..num_skills] of int: cook_skill_level;
  % Skill level (0-4) of each cook in each skill.  Row 0 = no-cook (all zeros).
array[1..num_tasks] of int: required_skill;
  % Skill index required by each task (1..num_skills).  -1 means no skill.
array[1..num_tasks] of int: min_level;
  % Minimum skill level (0-4) the assigned cook must meet.

% Pre-heat data
% -------------
array[1..num_preheats] of int: preheat_tasks;
  % Task index of each synthetic pre-heat task.
array[1..num_preheats] of int: preheat_bakes;
  % Task index of the bake task corresponding to each pre-heat.



% Decision variables
% ==================

% Start time of each task (minutes from time 0).
array[1..num_tasks] of var 0..horizon: start;

% Cook assigned to each task.  0 = no cook (set-and-forget task).
% 1..num_cooks = index of the assigned real cook.
array[1..num_tasks] of var 0..num_cooks: cook;

% Equipment assigned to each (task, resource-slot).  0 = no equipment.
array[1..num_tasks, 1..max_resources] of var 0..num_equipment: assign;

% Effective duration after applying skill-based overrides.  Derived from
% `eff_duration[cook[t], t]` when `needs_cook[t]` is true, otherwise
% just `duration[t]`.
array[1..num_tasks] of var 0..horizon: actual_duration;



% Derived variables
% =================

% recipe_end: Completion time of each recipe (latest finish among its tasks).
array[1..num_recipes] of var 0..horizon: recipe_end;

% Overall makespan — the time when the last task finishes.
var 0..horizon: max_end = max(recipe_end);

% Earliest recipe completion time (for the alignment objective).
var 0..horizon: min_recipe_end = min(recipe_end);

% Total idle time between each pre-heat finishing and its bake starting.
% Minimising this tucks pre-heats close to their bakes.
var 0..horizon: total_waste =
  if num_preheats > 0 then
    sum(p in 1..num_preheats)(
      start[preheat_bakes[p]] -
      (start[preheat_tasks[p]] + actual_duration[preheat_tasks[p]])
    )
  else 0 endif;



% Constraints
% ===========

% Duration selection
% ------------------
% Pick the effective duration based on whether the task needs a cook.
% Tasks needing a cook use the pre-computed `eff_duration` matrix entry
% for the assigned cook.  Set-and-forget tasks use the base duration.
constraint forall(t in 1..num_tasks)(
  actual_duration[t] = if needs_cook[t]
    then eff_duration[cook[t], t]
    else duration[t] endif
);

% Dependency chains
% -----------------
% A child task cannot start until its parent has finished.  Edges are
% (parent, child) pairs supplied as `deps_from` / `deps_to`.
constraint forall(i in 1..num_deps)(
  start[deps_to[i]] >= start[deps_from[i]] + actual_duration[deps_from[i]]
);

% Resource disjunction
% --------------------
% No two tasks may use the same equipment at the same time.  This is a
% pairwise no-overlap constraint over all (task, resource-slot) pairs.
% The `assign > 0` guard filters unused slots.
constraint forall(
  i in 1..num_tasks, j in 1..num_tasks where i < j,
  ri in 1..max_resources, rj in 1..max_resources
)(
  (assign[i, ri] > 0 /\ assign[i, ri] = assign[j, rj]) ->
  (start[i] + actual_duration[i] <= start[j] \/
   start[j] + actual_duration[j] <= start[i])
);

% Equipment kind matching
% -----------------------
% Each task's resource slot must be assigned to equipment whose kind matches
% the required kind.  Equipment of the same kind occupies a contiguous index
% range [kind_start[k], kind_end[k]].  Unused slots (kind 0) are forced to 0.
constraint forall(t in 1..num_tasks, r in 1..max_resources where task_kinds[t, r] > 0)(
  assign[t, r] >= kind_start[task_kinds[t, r]] /\
  assign[t, r] <= kind_end[task_kinds[t, r]]
);
constraint forall(t in 1..num_tasks, r in 1..max_resources where task_kinds[t, r] == 0)(
  assign[t, r] == 0
);

% Cook allocation
% ---------------
% Tasks that need a cook must be assigned to a real cook (> 0).
% Set-and-forget tasks (oven, rest) always get cook 0.
constraint forall(t in 1..num_tasks where needs_cook[t])(cook[t] > 0);
constraint forall(t in 1..num_tasks where not needs_cook[t])(cook[t] = 0);

% Cook disjunction
% ----------------
% Two active-cook tasks assigned to the same cook cannot overlap in time.
% Set-and-forget tasks do NOT occupy the cook (enables interleaving during
% oven baking).
constraint forall(
  i in 1..num_tasks, j in 1..num_tasks
  where i < j /\ needs_cook[i] /\ needs_cook[j] /\ cook[i] = cook[j]
)(
  start[i] + actual_duration[i] <= start[j] \/
  start[j] + actual_duration[j] <= start[i]
);

% Skill eligibility
% -----------------
% If a task requires a skill, the assigned cook must meet the minimum
% skill level.  The `cook_skill_level` matrix is pre-computed in Rust.
constraint forall(t in 1..num_tasks where required_skill[t] > 0)(
  cook_skill_level[cook[t], required_skill[t]] >= min_level[t]
);

% Recipe completion
% -----------------
% Each recipe finishes when its last task completes.
constraint forall(r in 1..num_recipes)(
  recipe_end[r] = max(
    [start[t] + actual_duration[t] | t in 1..num_tasks where recipe_of[t] = r]
  )
);



% Objective
% =========

% Weighted scalar combination with strict priority:
%   1. Makespan (`max_end`) — dominates via `(horizon + 1)` multiplier.
%   2. Alignment (`max_end - min_recipe_end`) — minimises the spread
%      between earliest and latest recipe completion so dishes finish
%      around the same time.
%   3. Pre-heat waste (`total_waste`) — minimises idle time between
%      pre-heat completion and bake start.
%
% The `(horizon + 1)` factor ensures no lower-priority term can outweigh
% a higher-priority one, regardless of actual values.  The
% `(1 + num_preheats)` factor extends this guarantee when the waste term
% is active, since `total_waste` can be at most `num_preheats × horizon`.
%
% A lexicographic objective (`minimize max_end ++ ...`) would be more
% idiomatic, but MiniZinc 2.9.3's `++` operator caused type errors with
% `var int` under Gecode.
solve minimize
  max_end * (horizon + 1) * (1 + num_preheats) +
  (max_end - min_recipe_end) * (1 + num_preheats) +
  total_waste;



% Output
% ======

% Produces MiniZinc's `--json-stream` output containing three arrays:
%   - `start[t]`:    start offset of each task (minutes)
%   - `cook[t]`:     cook assignment (0 = none, 1..num_cooks = real cook)
%   - `assign[t,r]`: equipment assignment (0 = none)
%
% The Rust parser (`parse_solution` in schedule.rs) reconstructs the Plan
% from these arrays.
output [
  "start = ", show(start), ";\n",
  "cook = ", show(cook), ";\n",
  "assign = ", show([assign[t, r] | t in 1..num_tasks, r in 1..max_resources]), ";\n"
];
