{ lib, rustPlatform }:
rustPlatform.buildRustPackage {
  pname = "makky";
  version = "0.1.0";

  src = lib.fileset.toSource {
    root = ../.;
    fileset = lib.fileset.intersection (lib.fileset.fromSource (lib.sources.cleanSource ../.)) (
      lib.fileset.unions [
        ../Cargo.toml
        ../Cargo.lock
        ../src
        ../LICENSE
      ]
    );
  };
  cargoLock.lockFile = ../Cargo.lock;

  meta = {
    description = "A dead simple tool to manage files in NixOS";
    homepage = "https://github.com/rossnomann/makky";
    license = lib.licenses.mit;
    mainProgram = "makky";
  };
}
