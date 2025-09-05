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
      system.build.sunwet = import ./package.nix {
        pkgs = pkgs;
        lib = lib;
      };
    };
  }
)
