{ package }:
{
  config,
  lib,
  pkgs,
  ...
}:
let
  cfg = config.makky;
in
{
  options.makky = {
    enable = lib.mkEnableOption "makky";
    files = lib.mkOption {
      type = lib.types.attrsOf (
        lib.types.submodule (
          { name, ... }:
          {
            options = {
              source = lib.mkOption { type = lib.types.path; };
              target = lib.mkOption { type = lib.types.str; };
            };
            config = {
              target = lib.mkDefault name;
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
          cmdRegister = "${cfg.executablePath} register";
          packageFiles = pkgs.runCommandLocal "makky-files" { } (
            ''
              mkdir -p $out
            ''
            + lib.strings.concatStrings (
              lib.mapAttrsToList (n: v: ''
                ${cmdRegister} $out/makky.metadata ${
                  lib.escapeShellArgs [
                    v.source
                    v.target
                  ]
                }
              '') cfg.files
            )
          );
        in
        {
          environment.systemPackages = [ packageFiles ];
          system.userActivationScripts.makkyLink =
            let
              metadataStorePath = "${packageFiles}/makky.metadata";
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
