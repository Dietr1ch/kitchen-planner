% Kitchen Planner — Scheduling Model
% Parameters supplied via DZN data (see schedule.rs)

int: num_tasks;
int: horizon;
int: num_cooks;
int: num_recipes;
int: num_deps;
int: num_equipment;
int: num_kinds;
int: max_resources;
int: num_skills;
int: num_preheats;

array[1..num_tasks] of int: duration;
array[1..num_tasks] of bool: needs_cook;
array[1..num_tasks] of int: recipe_of;
array[1..num_deps] of int: deps_from;
array[1..num_deps] of int: deps_to;
array[1..num_equipment] of int: equip_kind;
array[1..num_tasks, 1..max_resources] of int: task_kinds;
array[1..num_kinds] of int: kind_start;
array[1..num_kinds] of int: kind_end;
array[0..num_cooks, 1..num_tasks] of int: eff_duration;
array[0..num_cooks, 1..num_skills] of int: cook_skill_level;
array[1..num_tasks] of int: required_skill;
array[1..num_tasks] of int: min_level;
array[1..num_preheats] of int: preheat_tasks;
array[1..num_preheats] of int: preheat_bakes;

array[1..num_tasks] of var 0..horizon: start;
array[1..num_tasks] of var 0..num_cooks: cook;
array[1..num_tasks, 1..max_resources] of var 0..num_equipment: assign;

array[1..num_tasks] of var 0..horizon: actual_duration;
constraint forall(t in 1..num_tasks)(
  actual_duration[t] = if needs_cook[t] then eff_duration[cook[t], t] else duration[t] endif
);

constraint forall(i in 1..num_deps)(
  start[deps_to[i]] >= start[deps_from[i]] + actual_duration[deps_from[i]]
);

constraint forall(i in 1..num_tasks, j in 1..num_tasks where i < j, ri in 1..max_resources, rj in 1..max_resources)(
  (assign[i, ri] > 0 /\ assign[i, ri] = assign[j, rj]) ->
  (start[i] + actual_duration[i] <= start[j] \/ start[j] + actual_duration[j] <= start[i])
);

constraint forall(t in 1..num_tasks, r in 1..max_resources where task_kinds[t, r] > 0)(
  assign[t, r] >= kind_start[task_kinds[t, r]] /\ assign[t, r] <= kind_end[task_kinds[t, r]]
);
constraint forall(t in 1..num_tasks, r in 1..max_resources where task_kinds[t, r] == 0)(
  assign[t, r] == 0
);

constraint forall(t in 1..num_tasks where needs_cook[t])(cook[t] > 0);
constraint forall(t in 1..num_tasks where not needs_cook[t])(cook[t] = 0);

constraint forall(i in 1..num_tasks, j in 1..num_tasks where i < j /\ needs_cook[i] /\ needs_cook[j] /\ cook[i] = cook[j])(
  start[i] + actual_duration[i] <= start[j] \/ start[j] + actual_duration[j] <= start[i]
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

output ["start = ", show(start), ";\ncook = ", show(cook), ";\nassign = ", show([assign[t, r] | t in 1..num_tasks, r in 1..max_resources]), ";\n"];
