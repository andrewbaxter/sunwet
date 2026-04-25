{
  debug ? true,
}:
let
  pkgs = import <nixpkgs> { };
  lib = import (<nixpkgs> + "/lib");
  hoj =
    import
      (
        (fetchTarball "https://github.com/andrewbaxter/hammer-of-json/archive/4622456e0eeffd62380dbd88d648c28c8a3359d9.zip")
        + "/source/package.nix"
      )
      {
        pkgs = pkgs;
        lib = lib;
      };
  shared = import ./nixbuild/shared.nix {
    pkgs = pkgs;
    lib = lib;
  };
  wasmUnbound = shared.naersk.buildPackage {
    pname = "sunwet-browser";
    name = "sunwet-browser";
    root = shared.stageWorkspace "stage-browser-workspace" [
      ./browser
      ./browser/Cargo.lock
      ./browser/.cargo
      ./shared-wasm
      ./shared
      ./nixbuild/browser/Cargo.toml
    ];
    release = !debug;
    GIT_HASH = "fakehash";
    CARGO_BUILD_TARGET = "wasm32-unknown-unknown";
  };
  extensionIdKeyChrome = "TODO";
  extensionIdChrome = "TODO";
  extensionIdFirefox = "sunwet@example.org";
in
{
  extensionUnpacked = pkgs.runCommand "sunwet-browser-unpacked" { } ''
    hoj_cp () {
      ${pkgs.coreutils}/bin/cp -r --no-preserve=all "$@"
    }
    hoj_merge () {
      ${hoj}/bin/hoj "f:$1" merge "f:$2"
    }
    hoj_set () {
      ${hoj}/bin/hoj --in-place "f:$1" search-set "\"$2\"" "\"$3\""
    }

    ${pkgs.coreutils}/bin/mkdir -p $out
    ${pkgs.coreutils}/bin/mkdir -p stage
    hoj_cp ${./browser} browser_src
    hoj_cp ${./wasm/prestatic/big-icon.svg} browser_src/ext_static/big-icon.svg

    ${pkgs.coreutils}/bin/mkdir -p browser_wasm
    ${shared.nativeBindgen}/bin/bind_wasm --in-wasm ${wasmUnbound}/bin/browser-content.wasm --out-name content2 --out-dir browser_wasm
    ${shared.nativeBindgen}/bin/bind_wasm --in-wasm ${wasmUnbound}/bin/browser-options.wasm --out-name options2 --out-dir browser_wasm
    (cd browser_src/ext_static && ${pkgs.typescript}/bin/tsc --noEmit)

    version=$(${pkgs.gnugrep}/bin/grep "^version =" ${./shared/Cargo.toml} | ${pkgs.gnused}/bin/sed -e "s/.*\"\(.*\)\".*/\1/")
    hoj_set browser_src/browser_manifest.json _PLACEHOLDER_VERSION "$version"

    hoj_cp browser_src/ext_static stage/browser_chrome
    hoj_cp browser_wasm/* stage/browser_chrome/
    chrome_browser_manifest_path=stage/browser_chrome/manifest.json
    hoj_merge browser_src/browser_manifest.json ./browser_src/browser_manifest_chrome.json > $chrome_browser_manifest_path
    hoj_set $chrome_browser_manifest_path _PLACEHOLDER_BROWSERIDKEY '${extensionIdKeyChrome}'
    (cd stage/browser_chrome; ${pkgs.zip}/bin/zip $out/sunwet_chrome.zip --recurse-paths *)

    hoj_cp browser_src/ext_static stage/browser_firefox
    hoj_cp browser_wasm/* stage/browser_firefox/
    firefox_browser_manifest_path=stage/browser_firefox/manifest.json
    hoj_merge browser_src/browser_manifest.json ./browser_src/browser_manifest_firefox.json > $firefox_browser_manifest_path
    hoj_set $firefox_browser_manifest_path _PLACEHOLDER_BROWSERID '${extensionIdFirefox}'
    ${pkgs.web-ext}/bin/web-ext lint --output json --pretty --self-hosted --source-dir stage/browser_firefox
    (cd stage/browser_firefox; ${pkgs.zip}/bin/zip $out/sunwet_firefox.zip --recurse-paths *)
  '';
}
