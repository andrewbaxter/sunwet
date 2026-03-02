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
          };
        in
        pkgs.dockerTools.buildImage {
          name = "sunwet";
          config = {
            Cmd = [ "${sunwet}/bin/sunwet" ];
          };
        };
    };
  }
)
