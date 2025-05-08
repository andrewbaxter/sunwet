{ pkgs, lib }:
let
  fenix =
    import
      (fetchTarball "https://github.com/nix-community/fenix/archive/1a79901b0e37ca189944e24d9601c8426675de50.zip")
      { };
  toolchain = fenix.combine [
    fenix.stable.rustc
    fenix.stable.cargo
    fenix.targets.wasm32-unknown-unknown.stable.rust-std
  ];
  naersk =
    pkgs.callPackage
      #(fetchTarball "https://github.com/nix-community/naersk/archive/378614f37a6bee5a3f2ef4f825a73d948d3ae921.zip")
      ../../../../../3rd/naersk
      {
        rustc = toolchain;
        cargo = toolchain;
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
  workspaceWasm = stageWorkspace "sunwet-wasm" [
    ./nixbuild/wasm/Cargo.toml
    ./wasm/Cargo.lock
    ./wasm/.cargo
    ./wasm
    ./shared
  ];
  wasm = naersk.buildPackage {
    pname = "sunwet-wasm";
    root = workspaceWasm;
    release = false;
    CARGO_BUILD_TARGET = "wasm32-unknown-unknown";
  };
  nativeBindgen = naersk.buildPackage {
    root = ./native-bindgen;
  };
  static = pkgs.runCommand "sunwet-static" { } ''
    set -xeu -o pipefail
    ${pkgs.coreutils}/bin/mkdir -p $out
    ${nativeBindgen}/bin/bind_wasm --in-wasm ${wasm}/bin/nonlink.wasm --out-name nonlink --out-dir $out
    ${nativeBindgen}/bin/bind_wasm --in-wasm ${wasm}/bin/link.wasm --out-name link --out-dir $out
    ${pkgs.coreutils}/bin/cp -r ${./wasm}/static/* $out/
    ${pkgs.ffmpeg}/bin/ffmpeg -f lavfi -i anullsrc=r=11025:cl=mono -t 0.1 -acodec mp3 $out/audiotest.mp3
    ${pkgs.ffmpeg}/bin/ffmpeg -f lavfi -i anullsrc=r=11025:cl=mono -f lavfi -i "color=c=black:size=1x1" -t 0.1 $out/videotest.webm
  '';
  workspaceNative = stageWorkspace "sunwet-native" [
    ./nixbuild/native/Cargo.toml
    ./native/Cargo.lock
    ./native
    ./shared
  ];
  native = naersk.buildPackage {
    pname = "sunwet-native";
    root = workspaceNative;
    release = false;
    STATIC_DIR = "${static}";
    GIT_HASH = "fakehash";
    buildInputs = [ pkgs.sqlite ];
  };
in
native
