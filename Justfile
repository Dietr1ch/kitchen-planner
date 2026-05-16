check:
	ron-lsp check
	cargo check \
	  --all-targets
	nix flake check

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
	static-web-server --config-file .config/server.toml
