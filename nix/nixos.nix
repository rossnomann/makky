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
              diff = "${pkgs.diffutils}/bin/diff";
            in
            ''
              function __makky_activate() {
                local metadata_actual=${cfg.metadataPath}
                local metadata_store=${metadataStorePath}
                local target_root=${cfg.targetRoot}
                local makky_executable=${cfg.executablePath}
                local diff_executable=${diff}
                local do_unlink=false
                local do_link=false

                if [ -f $metadata_actual ]; then
                  set +e
                  $diff_executable -q $metadata_actual $metadata_store
                  diff_status=$?
                  set -e
                  if [ $diff_status -ne 0 ]; then
                    do_unlink=true
                    do_link=true
                  fi
                else
                  do_link=true
                fi

                if [ $do_unlink = true ]; then
                  $makky_executable unlink $metadata_actual $target_root
                  rm -f $metadata_actual
                fi

                if [ $do_link = true ]; then
                  cp $metadata_store $metadata_actual
                  $makky_executable link $metadata_store $target_root
                fi
              }

              __makky_activate
            '';
        }
      );
}
