# Makky

A dead simple tool to manage files in NixOS. Use at your own risk.

## Usage

`flake.nix`:
```nix
{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    makky = {
      url = "github:rossnomann/makky";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };
  outputs = inputs: {
    nixosConfigurations.default = inputs.nixpkgs.lib.nixosSystem {
      system = "x86_64-linux";
      modules = [
        inputs.makky.nixosModules.default
        (
          { config, ... }:
          {
            config = {
              makky = {
                enable = true;
                targetRoot = "$HOME";
                metadataPath = "$HOME/.config/makky.metadata";
                files = {
                  ".config/git/config".source = ./resources/git/config;
                };
              };
            };
          }
        )
      ];
    };
  };
}
```

## Limitations

It only creates/removes symlinks. Everything else is up to you.

## LICENSE

The MIT License (MIT)
