{ pkgs, lib }:
let
  fenix =
    import
      (fetchTarball "https://github.com/nix-community/fenix/archive/77de5067629e201436c76f14f96614a19368c4ae.zip")
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
    release = true;
    CARGO_BUILD_TARGET = "wasm32-unknown-unknown";
    GIT_HASH = "fakehash";
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
    ${pkgs.inkscape}/bin/inkscape --export-filename $out/pwa-icon-512.png --export-width 512 ${./wasm}/prestatic/big-icon.svg
    ${pkgs.inkscape}/bin/inkscape --export-filename $out/pwa-icon-192.png --export-width 192 ${./wasm}/prestatic/big-icon.svg
    ${pkgs.inkscape}/bin/inkscape --export-filename $out/apple-icon-180.png --export-width 180 ${./wasm}/prestatic/small-icon.svg
    ${pkgs.inkscape}/bin/inkscape --export-filename $out/favicon.png --export-width 128 ${./wasm}/prestatic/small-icon.svg
  '';

  workspaceNative = stageWorkspace "sunwet-native" [
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
  native = naersk.buildPackage {
    pname = "sunwet-native";
    root = workspaceNative;
    release = false;
    STATIC_DIR = "${static}";
    GIT_HASH = "fakehash";
    nativeBuildInputs = [
      pkgs.makeWrapper
    ];
    buildInputs = [ pkgs.sqlite ];
    postInstall = wrapBinary {
      bin = "sunwet";
      packages = [
        pkgs.ffmpeg
        pkgs.pandoc
        pkgs._7zz
        pkgs.mkvtoolnix
      ];
    };
  };
in
native
