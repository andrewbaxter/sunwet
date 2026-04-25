{
  pkgs,
  lib,
  # Include mkvtoolnix for the cli media scan import functionality (adds large qt dependency, optional to reduce docker image size)
  cli-import ? true,
  debug ? true,
}:
let
  shared = (import ./nixbuild/shared.nix) { pkgs=pkgs; lib=lib; };
  workspaceWasm = shared.stageWorkspace "sunwet-wasm" [
    ./nixbuild/wasm/Cargo.toml
    ./wasm/Cargo.lock
    ./wasm/.cargo
    ./wasm
    ./shared
    ./shared-wasm
  ];
  wasm = shared.naersk.buildPackage {
    pname = "sunwet-wasm";
    name = "sunwet-wasm"; # For nix build error messages only
    root = workspaceWasm;
    release = true;
    CARGO_BUILD_TARGET = "wasm32-unknown-unknown";
    GIT_HASH = "fakehash";
  };
  static = pkgs.runCommand "sunwet-static" { } ''
    set -xeu -o pipefail
    ${pkgs.coreutils}/bin/mkdir -p $out
    ${shared.nativeBindgen}/bin/bind_wasm --in-wasm ${wasm}/bin/nonlink.wasm --out-name nonlink --out-dir $out
    ${shared.nativeBindgen}/bin/bind_wasm --in-wasm ${wasm}/bin/link.wasm --out-name link --out-dir $out
    ${pkgs.coreutils}/bin/cp -r ${./wasm}/static/* $out/
    ${pkgs.ffmpeg}/bin/ffmpeg -f lavfi -i anullsrc=r=11025:cl=mono -t 0.1 -acodec mp3 $out/audiotest.mp3
    ${pkgs.ffmpeg}/bin/ffmpeg -f lavfi -i anullsrc=r=11025:cl=mono -f lavfi -i "color=c=black:size=1x1" -t 0.1 $out/videotest.webm
    ${pkgs.inkscape}/bin/inkscape --export-filename $out/pwa-icon-512.png --export-width 512 ${./wasm}/prestatic/big-icon.svg
    ${pkgs.inkscape}/bin/inkscape --export-filename $out/pwa-icon-192.png --export-width 192 ${./wasm}/prestatic/big-icon.svg
    ${pkgs.inkscape}/bin/inkscape --export-filename $out/apple-icon-180.png --export-width 180 ${./wasm}/prestatic/big-icon.svg
    ${pkgs.inkscape}/bin/inkscape --export-filename $out/favicon.png --export-width 128 ${./wasm}/prestatic/small-icon.svg
  '';

  workspaceNative = shared.stageWorkspace "sunwet-native" [
    ./nixbuild/native/Cargo.toml
    ./native/Cargo.lock
    ./native
    ./shared
  ];
  wrapBinary =
    {
      bin,
      packages ? null,
      libraries ? null,
    }:
    let
      path = (
        lib.strings.concatStringsSep ":" (
          [ "$out/bin" ] ++ (map (p: "${p}/bin") (if packages != null then packages else [ ]))
        )
      );
      wrapArgs =
        [ ]
        ++ [ "--prefix PATH : ${path}" ]
        ++ (lib.lists.optionals (libraries != null) [
          "--prefix LD_LIBRARY_PATH : ${lib.makeLibraryPath libraries}"
        ]);
    in
    ''
      wrapProgram $out/bin/${bin} ${lib.strings.concatStringsSep " " wrapArgs}
    '';
  native = shared.naersk.buildPackage {
    pname = "sunwet-native";
    name = "sunwet-native"; # For nix build error messages only
    root = workspaceNative;
    release = !debug;
    STATIC_DIR = "${static}";
    GIT_HASH = "fakehash";
    nativeBuildInputs = [
      pkgs.makeWrapper
    ];
    buildInputs = [ pkgs.sqlite ];
    postInstall = wrapBinary {
      bin = "sunwet";
      packages = [
        pkgs.ffmpeg-headless
        pkgs.pandoc
        pkgs._7zz
      ]
      ++ (if cli-import then [ pkgs.mkvtoolnix-cli ] else [ ]);
    };
  };
in
native
