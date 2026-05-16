build:
	nix build

check:
	ron-lsp check
	nix flake check

plan:
	@cargo run --quiet --bin schedule -- schedule data/kitchens/simple.ron data/cooks data/recipes/*.ron

gantt path:
	@cargo run --quiet --bin gantt -- {{path}}

run:
	@just plan | just gantt -

run-html path:
	@cargo run --quiet --bin schedule -- schedule data/kitchens/simple.ron data/cooks data/recipes/*.ron | cargo run --quiet --bin gantt -- --format html - > {{path}}

serve:
	@echo "Open http://localhost:3004 in your browser"
	static-web-server -c .config/server.toml
