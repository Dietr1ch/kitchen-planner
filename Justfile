check: export_ts_types
	ron-lsp check
	tsc --noEmit
	cargo check \
	  --all-targets
	nix flake check

export_ts_types:
	cargo test export_bindings

fmt:
	cargo fmt

lint:
	cargo clippy \
	  --all-targets \
	  --fix \
	  --allow-dirty

build:
	nix build


plan +ARGS:
	@cargo run \
	  --quiet \
	  --bin schedule \
	  -- \
	  {{ARGS}}

gantt path:
	@cargo run --quiet --bin gantt -- {{path}}

run +ARGS:
	@just plan {{ARGS}} | just gantt -


serve:
	@echo "Open http://localhost:3004 in your browser"
	cargo run --quiet --bin web
