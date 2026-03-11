let
  nixpkgsPath = <nixpkgs>;
  buildSystem = (
    configuration: import (nixpkgsPath + /nixos/lib/eval-config.nix) { modules = [ configuration ]; }
  );
in
buildSystem (
  { pkgs, lib, ... }:
  {
    config = {
      system.build.sunwet =
        let
          sunwet = import ./package.nix {
            pkgs = pkgs;
            lib = lib;
            cli-import = false;
          };
          volPersistent = "/vol/persistent";
          volCache = "/vol/cache";
          config = derivation {
            name = "sunwet-singleuser-config";
            system = builtins.currentSystem;
            builder = "${pkgs.bash}/bin/bash";
            args = [
              (pkgs.writeShellScript "sunwet-config-script" ''
                set -xeu -o pipefail
                export PATH="$PATH":${sunwet}/bin
                export SUNWET_PERSISTENT_DIR="${volPersistent}"
                export SUNWET_CACHE_DIR="${volCache}"
                export SUNWET_TOKEN=sunwet
                export SUNWET_BIND_ADDR=0.0.0.0:8080
                ${pkgs.nodejs}/bin/node ${./.}/local_example.ts
                ${pkgs.coreutils}/bin/mv ./config.json $out
              '')
            ];
            checkPhase = ''
              ${sunwet}/bin/sunwet run-server --validate $out
            '';
          };
        in
        pkgs.dockerTools.buildImage {
          name = "sunwet";
          config = {
            Entrypoint = [ "${sunwet}/bin/sunwet" ];
            Cmd = [
              "run-server"
              "${config}"
            ];
            ExposedPorts = {
              "8080/tcp" = { };
            };
            Volumes = {
              ${volPersistent} = { };
              ${volCache} = { };
            };
          };
        };
    };
  }
)
