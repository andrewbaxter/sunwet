#!/usr/bin/env bash
set -xeuo pipefail
nix-build browser.nix -o ./built_ext --arg debug true
