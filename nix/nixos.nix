{ package }:
{
  config,
  lib,
  pkgs,
  ...
}:
let
  cfg = config.makky;
  utils = import ./utils.nix { inherit lib; };
in
{
  options.makky = {
    enable = lib.mkEnableOption "makky";
    files = lib.mkOption {
      type = lib.types.attrsOf (
        lib.types.submodule (
          { name, config, ... }:
          {
            options = {
              store = {
                name = lib.mkOption {
                  type = lib.types.str;
                };
                path = lib.mkOption {
                  type = lib.types.path;
                };
              };
              source = lib.mkOption {
                type = lib.types.path;
              };
              text = lib.mkOption {
                type = lib.types.nullOr lib.types.lines;
                default = null;
              };
              executable = lib.mkOption {
                type = lib.types.bool;
                default = false;
              };
              target = lib.mkOption { type = lib.types.str; };
            };
            config =
              let
                storeName = utils.mkStoreName "makky_" name;
              in
              {
                store = {
                  name = lib.mkDefault storeName;
                  path = lib.mkDefault (utils.mkStorePath "makky_" config.source);
                };
                target = lib.mkDefault name;
                source = lib.mkIf (config.text != null) (
                  lib.mkDefault (
                    pkgs.writeTextFile {
                      name = storeName;
                      executable = config.executable == true;
                      text = config.text;
                    }
                  )
                );
              };
          }
        )
      );
      default = { };
    };
    targetRoot = lib.mkOption { type = lib.types.str; };
    metadataPath = lib.mkOption { type = lib.types.str; };

    package = lib.mkOption {
      type = lib.types.package;
      default = package;
    };
    executablePath = lib.mkOption {
      type = lib.types.str;
      default = "${cfg.package}/bin/makky";
    };
  };
  config =
    lib.mkIf (cfg.enable && cfg.files != { } && cfg.targetRoot != null && cfg.metadataPath != null)
      (
        let
          packageFiles =
            pkgs.runCommandLocal "makky-files"
              {
              }
              (
                let
                  register = "${cfg.executablePath} register";
                  registerFiles = lib.strings.concatStrings (
                    lib.mapAttrsToList (n: v: ''
                      ${register} $out/share/makky/makky.metadata ${
                        lib.escapeShellArgs [
                          v.store.path
                          v.target
                        ]
                      }
                    '') cfg.files
                  );
                in
                ''
                  mkdir -p $out/share/makky
                  ${registerFiles}
                ''
              );
        in
        {
          environment.systemPackages = [ packageFiles ];
          system.userActivationScripts.makkyLink =
            let
              metadataStorePath = "${packageFiles}/share/makky/makky.metadata";
            in
            ''
              if [ -f ${cfg.metadataPath} ]; then
                ${cfg.executablePath} unlink ${cfg.metadataPath} ${cfg.targetRoot}
                rm -f ${cfg.metadataPath}
              fi
              cp ${metadataStorePath} ${cfg.metadataPath}
              ${cfg.executablePath} link ${metadataStorePath} ${cfg.targetRoot}
            '';
        }
      );
}
