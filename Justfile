check:
	ron-lsp check
	cargo check \
	  --all-features \
	  --all-targets
	nix flake check

fmt:
	cargo fmt

lint:
	cargo clippy \
	  --all-features \
	  --all-targets \
	  --fix \
	  --allow-dirty

build:
	nix build

plan:
	@cargo run \
	  --quiet \
	  --bin schedule \
	  -- \
	  data/kitchens/simple.ron \
	  --cook data/cooks/alice.ron \
	  --cook data/cooks/bob.ron \
	  --cook data/cooks/charlie.ron \
	  --cook data/cooks/diana.ron \
	  data/recipes/*.ron

gantt path:
	@cargo run --quiet --bin gantt -- {{path}}

run:
	@just plan | just gantt -

run-html path:
	@cargo run \
	  --quiet \
	  --bin schedule \
	  -- \
	  data/kitchens/simple.ron \
	  --cook data/cooks/alice.ron \
	  --cook data/cooks/bob.ron \
	  --cook data/cooks/charlie.ron \
	  --cook data/cooks/diana.ron \
	  data/recipes/*.ron \
	  | cargo run \
	      --quiet \
	      --bin gantt \
	      -- \
	      --format html \
	      - \
	  > {{path}}

serve:
	@echo "Open http://localhost:3004 in your browser"
	static-web-server --config-file .config/server.toml
