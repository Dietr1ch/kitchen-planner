{
  pkgs ? import <nixpkgs> { },
  rustToolchain,
}:

pkgs.mkShell {
  buildInputs = with pkgs; [
    nixpkgs-fmt

    rustToolchain
    bacon
    cargo-nextest
  ];

  # Environment
  "RUST_SRC_PATH" = "${rustToolchain}/lib/rustlib/src/rust/library";

  shellHook = ''
    cargo --version
    cargo-nextest --version
  '';
}
