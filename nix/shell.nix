{ pkgs }:
let
  rust-dev = (
    pkgs.rust-bin.selectLatestNightlyWith (
      toolchain:
      toolchain.minimal.override {
        extensions = [
          "rust-analyzer"
          "rust-src"
          "rustfmt"
        ];
      }
    )
  );
in
pkgs.mkShell {
  RUST_SRC_PATH = "${rust-dev}/lib/rustlib/src/rust/library";
  buildInputs = [
    (pkgs.lib.hiPrio (
      pkgs.rust-bin.stable.latest.minimal.override {
        extensions = [
          "rust-docs"
          "clippy"
          "llvm-tools"
        ];
      }
    ))
    rust-dev
  ];
  shellHook = ''
    export CARGO_HOME="$PWD/.cargo"
    export PATH="$CARGO_HOME/bin:$PATH"
    mkdir -p .cargo
    echo '*' > .cargo/.gitignore
    if ! [ -f .cargo/bin/cargo-llvm-cov ]; then
        cargo install cargo-llvm-cov --locked
    fi
    export RUST_BACKTRACE=full
  '';
}
