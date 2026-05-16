{
  pkgs ? import <nixpkgs> { },
  rustToolchain,
}:

pkgs.mkShell {
  buildInputs = with pkgs; [
    nixpkgs-fmt

    # Rust
    rustToolchain
    bacon
    cargo-nextest

    # Solver
    minizinc
    gecode

    # RON
    ron-lsp
  ];

  # Environment
  "RUST_SRC_PATH" = "${rustToolchain}/lib/rustlib/src/rust/library";

  shellHook = ''
    cargo --version
    cargo-nextest --version
  '';
}
