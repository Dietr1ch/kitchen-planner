build:
	nix build

check:
	nix flake check

plan:
	@cargo run --quiet --bin main -- schedule data/kitchen.json data/cooks data/recipes/*.json

gantt path:
	@cargo run --quiet --bin gantt -- {{path}}

run:
	@just plan | just gantt -

run-html path:
	cargo run --quiet --bin main -- schedule data/kitchen.json data/cooks data/recipes/*.json | cargo run --quiet --bin gantt -- --format html - > {{path}}
