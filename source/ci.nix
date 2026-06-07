{
  version ? "0.0.0",
}:
let
  docker = import ./docker.nix;
  browser = import ./browser.nix { debug = false; version = version; };
in
{
  dockerImage = docker.config.system.build.sunwet;
  firefoxExtension = browser.extensionUnpacked;
}
