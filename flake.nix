{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };
  outputs =
    inputs:
    let
      system = "x86_64-linux";
      overlays = [ inputs.rust-overlay.overlays.default ];
      pkgs = import inputs.nixpkgs { inherit system overlays; };
      package = inputs.self.packages.${system}.default;
    in
    {
      devShells.${system}.default = import ./nix/shell.nix { inherit pkgs; };
      nixosModules.default = import ./nix/nixos.nix { inherit package; };
      packages.${system}.default = import ./nix/package.nix { inherit (pkgs) lib rustPlatform; };
    };
}
