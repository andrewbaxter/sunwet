{ config, pkgs, lib, ... }:
let
  browser = import ./browser.nix { pkgs = pkgs; lib = lib; debug = true; };
in
{
  imports = [
    ./nixbuild/native
  ];
  config = {
    environment.etc = {
      # Locates binary, allows access from extensions
      "chromium/native-messaging-hosts/${browser.nativeId}.json".source = "${browser.extensionUnpacked}/native/manifest_chrome.json";
      "opt/chrome/native-messaging-hosts/${browser.nativeId}.json".source = "${browser.extensionUnpacked}/native/manifest_chrome.json";
      "opt/vivaldi/native-messaging-hosts/${browser.nativeId}.json".source = "${browser.extensionUnpacked}/native/manifest_vivaldi.json";
      "opt/brave/native-messaging-hosts/${browser.nativeId}.json".source = "${browser.extensionUnpacked}/native/manifest_brave.json";

      # Installs extension, not sure why there's only one id
      #"chromium/policies/managed/${nativeId}.json".source = "${extensionUnpacked}/browser/policy_chrome.json";
      #"opt/chrome/policies/managed/${nativeId}.json".source = "${extensionUnpacked}/browser/policy_chrome.json";
      #"opt/vivaldi/policies/managed/${nativeId}.json".source = "${extensionUnpacked}/browser/policy_chrome.json";
      #"opt/brave/policies/managed/${nativeId}.json".source = "${extensionUnpacked}/browser/policy_chrome.json";
    };

    # Locates binary, allows access from extensions
    programs.firefox.nativeMessagingHosts.packages = [ browser.extensionUnpacked ];

    # Make it available in packages
    nixpkgs.overlays = [
      (finalPkgs: prevPkgs: {
        passworthExtensionUnpacked = browser.extensionUnpacked;
      })
    ];
  };
}
