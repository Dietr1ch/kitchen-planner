build:
	nix build

check:
	nix flake check

plan:
	@cargo run --quiet --bin schedule -- schedule data/kitchen.json data/cooks data/recipes/*.json

gantt path:
	@cargo run --quiet --bin gantt -- {{path}}

run:
	@just plan | just gantt -

run-html path:
	@cargo run --quiet --bin schedule -- schedule data/kitchen.json data/cooks data/recipes/*.json | cargo run --quiet --bin gantt -- --format html - > {{path}}

serve:
	@echo "Open http://localhost:3004 in your browser"
	static-web-server -c .config/server.toml
