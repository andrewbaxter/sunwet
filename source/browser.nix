{
  pkgs,
  lib,
  debug ? true,
}:
let
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
  rust = import ./rust.nix {
    pkgs = pkgs;
    lib = lib;
  };

  wasmUnbound = import ./nixbuild/browser {
    pkgs = pkgs;
    lib = lib;
    debug = debug;
  };
  nativeBindgen = rust.naersk.buildPackage {
    root = ./native-bindgen;
  };
  nativeId = "me.isandrew.sunwet";
  extensionIdKeyChrome = "TODO";
  extensionIdChrome = "TODO";
  extensionIdFirefox = "sunwet@example.org";
in
{
  nativeId = nativeId;
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

    ${pkgs.coreutils}/bin/mkdir -p stage
    hoj_cp ${./browser} browser_src
    hoj_cp ${./wasm/prestatic/big-icon.svg} browser_src/ext_static/big-icon.svg

    # TypeScript type check JS files
    if command -v ${pkgs.nodejs}/bin/node >/dev/null 2>&1 && command -v ${pkgs.typescript}/bin/tsc >/dev/null 2>&1; then
      echo "Running TypeScript type checks..."
      (cd browser_src/ext_static && ${pkgs.typescript}/bin/tsc --noEmit)
    else
      echo "Warning: node or tsc not available, skipping TypeScript type checks"
    fi

    # Assemble browser bits
    ${pkgs.coreutils}/bin/mkdir -p browser_wasm
    ${nativeBindgen}/bin/bind_wasm --in-wasm ${wasmUnbound}/bin/browser-content.wasm --out-name content2 --out-dir browser_wasm
    ${nativeBindgen}/bin/bind_wasm --in-wasm ${wasmUnbound}/bin/browser-options.wasm --out-name options2 --out-dir browser_wasm

    version=$(${pkgs.gnugrep}/bin/grep "^version =" ${./shared/Cargo.toml} | ${pkgs.gnused}/bin/sed -e "s/.*\"\(.*\)\".*/\1/")
    hoj_set browser_src/browser_manifest.json _PLACEHOLDER_VERSION "$version"

    hoj_cp browser_src/ext_static stage/browser_chrome
    hoj_cp browser_wasm/* stage/browser_chrome/
    chrome_browser_manifest_path=stage/browser_chrome/manifest.json
    hoj_merge browser_src/browser_manifest.json ./browser_src/browser_manifest_chrome.json > $chrome_browser_manifest_path
    hoj_set $chrome_browser_manifest_path _PLACEHOLDER_BROWSERIDKEY '${extensionIdKeyChrome}'

    hoj_cp browser_src/ext_static stage/browser_firefox
    hoj_cp browser_wasm/* stage/browser_firefox/
    firefox_browser_manifest_path=stage/browser_firefox/manifest.json
    hoj_merge browser_src/browser_manifest.json ./browser_src/browser_manifest_firefox.json > $firefox_browser_manifest_path
    hoj_set $firefox_browser_manifest_path _PLACEHOLDER_BROWSERID '${extensionIdFirefox}'

    ${pkgs.web-ext}/bin/web-ext lint --output json --pretty --self-hosted --source-dir $out/browser_firefox
  '';
}
