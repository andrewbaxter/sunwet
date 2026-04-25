{
  pkgs,
  lib,
}: rec {
  fenix = import ./fenix { };
  toolchain = fenix.combine [
    fenix.stable.rustc
    fenix.stable.cargo
    fenix.targets.wasm32-unknown-unknown.stable.rust-std
  ];
  naersk =
    pkgs.callPackage
      ./naersk
      {
        rustc = toolchain;
        cargo = toolchain;
      };
  nativeBindgen = naersk.buildPackage {
    root = ../native-bindgen;
  };
  stageWorkspace =
    name: files:
    let
      linkLines = lib.strings.concatStringsSep "\n" (
        map (f: ''
          filename=$(${pkgs.coreutils}/bin/basename ${f} | ${pkgs.gnused}/bin/sed -e 's/[^-]*-//')
          ${pkgs.coreutils}/bin/cp -r ${f} $filename
        '') files
      );
    in
    pkgs.runCommand "stage-rust-workspace-${name}" { } ''
      set -xeu -o pipefail
      ${pkgs.coreutils}/bin/mkdir $out
      cd $out
      ${linkLines}
    '';
}
